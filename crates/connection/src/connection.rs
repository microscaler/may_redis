// Connection — connection loop, request queue, and response dispatch
//
// Mirrors the may_postgres Connection pattern:
// - Single go! coroutine running an epoll loop
// - mpsc Queue<Request> for sending commands from application coroutines
// - spsc Sender per-request for response dispatch
// - Monotonically increasing tags for request-response matching
// - Non-blocking read/write with BytesMut buffers
// - WaitIoWaker to wake the connection loop on new requests

use base::{RedisError, RedisValue};
use bytes::{BufMut, BytesMut};
use codec::reader::RESPReader;
use may::coroutine::JoinHandle;
use may::go;
use may::io::{WaitIo, WaitIoWaker};
use may::net::TcpStream;
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::{ConnectionError, TcpConnector};

/// A request to be sent to the Redis server.
/// Contains the serialized RESP bytes and a sender for the response.
pub struct Request {
    /// Serialized RESP bytes to send to the server.
    pub data: Vec<u8>,
    /// Channel sender to deliver the response back to the requesting coroutine.
    pub sender: spsc::Sender<RedisValue>,
}

impl Request {
    /// Create a new request with the given data and channel sender.
    #[must_use]
    pub const fn new(data: Vec<u8>, sender: spsc::Sender<RedisValue>) -> Self {
        Self { data, sender }
    }
}

/// Internal state tracked per pending request for response dispatch.
struct PendingRequest {
    sender: spsc::Sender<RedisValue>,
}

/// A connection to a Redis server running the connection loop coroutine.
pub struct Connection {
    /// Handle to the connection loop coroutine, used for graceful shutdown.
    io_handle: JoinHandle<()>,
    /// Shared request queue for pushing commands from application coroutines.
    req_queue: Arc<Queue<Request>>,
    /// Waker to signal the connection loop about new requests.
    waker: WaitIoWaker,
    /// Unique connection identifier (socket fd).
    id: usize,
    /// Monotonic tag counter for request-response matching.
    tag_counter: Arc<AtomicUsize>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let rx = self.io_handle.coroutine();
        unsafe { rx.cancel() };
    }
}

#[inline]
fn reserve_buf(buf: &mut BytesMut) {
    let rem = buf.capacity() - buf.len();
    if rem < 512 {
        buf.reserve(65536 - rem);
    }
}

/// Decode all complete RESP values from the buffer.
/// On decode error (incomplete data), the buffer is unchanged.
/// On parse error, the buffer is restored and Err is returned.
fn decode_responses(
    read_buf: &mut BytesMut,
    resp_queue: &mut VecDeque<PendingRequest>,
) -> io::Result<()> {
    while !read_buf.is_empty() {
        let mut reader = RESPReader::new(read_buf.split());
        match reader.read_value() {
            Ok(value) => {
                if let Some(pending) = resp_queue.pop_front() {
                    let _ = pending.sender.send(value);
                } else {
                    log::warn!("unexpected response from server");
                }
            }
            Err(RedisError::Parse(_)) => {
                // Incomplete data — wait for more.
                read_buf.unsplit(reader.take_buf());
                break;
            }
            Err(e) => {
                log::error!("decode error: {e}");
                read_buf.unsplit(reader.take_buf());
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
    }
    Ok(())
}

/// Spawn the epoll-based connection loop coroutine.
fn spawn_connection_loop(mut stream: TcpStream, req_queue: Arc<Queue<Request>>) -> JoinHandle<()> {
    go!(move || {
        let mut read_buf = BytesMut::with_capacity(65536);
        let mut write_buf = BytesMut::with_capacity(65536);
        let mut resp_queue = VecDeque::<PendingRequest>::with_capacity(512);
        let mut io_events = 1;

        loop {
            // Process any queued requests
            process_req(&req_queue, &mut resp_queue, &mut write_buf);

            // Flush write buffer to socket (may's write handles WouldBlock via cooperative yielding)
            if !write_buf.is_empty() {
                if stream.write(&write_buf).is_err() {
                    let e = std::io::Error::new(std::io::ErrorKind::Other, "write");
                    log::error!("write error: {e}");
                    while let Some(pending) = resp_queue.pop_front() {
                        let _ = pending
                            .sender
                            .send(RedisValue::Error(format!("Write error: {e}")));
                    }
                    break;
                }
                write_buf.clear();
            }

            // Read from socket if allowed
            if io_events & 1 != 0 {
                let mut tmp_buf = [0u8; 65536];
                match stream.read(&mut tmp_buf) {
                    Ok(0) => {
                        log::error!("connection closed by server");
                        while let Some(pending) = resp_queue.pop_front() {
                            let _ = pending
                                .sender
                                .send(RedisValue::Error("Connection closed".into()));
                        }
                        break;
                    }
                    Ok(n) => {
                        read_buf.extend_from_slice(&tmp_buf[..n]);
                    }
                    Err(e) => {
                        log::error!("read error: {e}");
                        while let Some(pending) = resp_queue.pop_front() {
                            let _ = pending
                                .sender
                                .send(RedisValue::Error(format!("Read error: {e}")));
                        }
                        break;
                    }
                }
            }

            // Decode responses from read buffer
            if let Err(e) = decode_responses(&mut read_buf, &mut resp_queue) {
                log::error!("decode error: {e}");
                while let Some(pending) = resp_queue.pop_front() {
                    let _ = pending
                        .sender
                        .send(RedisValue::Error(format!("Decode error: {e}")));
                }
                break;
            }

            // Wait for I/O events using epoll
            io_events = if io_events & 1 != 0 || write_offset < write_buf.len() {
                stream.wait_io()
            } else {
                1
            }
        }
    })
}

/// Process queued requests: add to response queue and write buffer.
fn process_req(
    queue: &Queue<Request>,
    resp_queue: &mut VecDeque<PendingRequest>,
    write_buf: &mut BytesMut,
) {
    while let Some(req) = queue.pop() {
        reserve_buf(write_buf);
        resp_queue.push_back(PendingRequest { sender: req.sender });
        write_buf.put_slice(&req.data);
    }
}

impl Connection {
    /// Establish a TCP connection to the Redis server and spawn the connection loop.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    ///
    /// # Returns
    /// A [`Connection`] instance with an active epoll loop running in a background coroutine.
    pub fn connect(host: &str, port: u16) -> Result<Self, ConnectionError> {
        let stream = TcpConnector::connect(host, port)?;

        let id = stream.as_raw_fd() as usize;
        let waker = stream.waker();
        let req_queue = Arc::new(Queue::new());

        let io_handle = spawn_connection_loop(stream, req_queue.clone());

        Ok(Self {
            io_handle,
            req_queue,
            waker,
            id,
            tag_counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Send a command to the Redis server.
    ///
    /// The command is pushed to the shared request queue and the connection loop
    /// is woken up to process it. Returns the tag assigned to this request.
    #[must_use]
    pub fn send(&self, request: Request) -> usize {
        let tag = self.tag_counter.fetch_add(1, Ordering::SeqCst);
        self.req_queue.push(request);
        self.waker.wakeup();
        tag
    }

    /// Returns the unique connection identifier (socket fd).
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }
}
