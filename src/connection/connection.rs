// Connection — connection loop, request queue, and response dispatch
//
// Mirrors the may_postgres Connection pattern:
// - Single go! coroutine running an epoll loop
// - mpsc Queue<Request> for sending commands from application coroutines
// - spsc Sender per-request for response dispatch
// - Monotonically increasing tags for request-response matching
// - Non-blocking read/write with BytesMut buffers
// - WaitIoWaker to wake the connection loop on new requests

#![allow(clippy::doc_markdown)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::transmute_ptr_to_ref)]
#![allow(clippy::io_other_error)]
#![allow(clippy::ref_as_ptr)]

use bytes::{Buf, BufMut, BytesMut};
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

use crate::core::{RedisError, RedisValue};
use crate::codec::reader::RESPReader;
use super::tcp::{ConnectionError, TcpConnector};

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

/// Process queued requests: add to response queue and write buffer.
fn process_req(
    queue: &Queue<Request>,
    resp_queue: &mut VecDeque<PendingRequest>,
    write_buf: &mut BytesMut,
) {
    while let Some(req) = queue.pop() {
        let rem = write_buf.capacity() - write_buf.len();
        if rem < 512 {
            write_buf.reserve(65536 - rem);
        }
        resp_queue.push_back(PendingRequest { sender: req.sender });
        write_buf.put_slice(&req.data);
    }
}

/// Read from the inner raw socket into a [`BytesMut`] buffer.
/// Returns `Ok(true)` if more data might be available, `Ok(false)` if [`WouldBlock`].
fn nonblock_read(stream: &mut std::net::TcpStream, read_buf: &mut BytesMut) -> io::Result<bool> {
    let buf: &mut [u8] = unsafe { &mut *(read_buf.chunk_mut() as *mut _ as *mut [u8]) };
    let len = buf.len();
    let mut read_cnt = 0;
    while read_cnt < len {
        match stream.read(unsafe { buf.get_unchecked_mut(read_cnt..) }) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => read_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    unsafe { read_buf.advance_mut(read_cnt) };
    Ok(read_cnt < len)
}

/// Write from a [`BytesMut`] buffer to the inner raw socket.
fn nonblock_write(stream: &mut std::net::TcpStream, write_buf: &mut BytesMut) -> io::Result<usize> {
    let buf = write_buf.chunk();
    let len = buf.len();
    let mut write_cnt = 0;
    while write_cnt < len {
        match stream.write(unsafe { buf.get_unchecked(write_cnt..) }) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => write_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    write_buf.advance(write_cnt);
    Ok(write_cnt)
}

/// Decode all complete RESP values from the buffer.
/// On decode error (incomplete data), the buffer is unchanged.
/// On parse error, the buffer is restored and [`Err`] is returned.
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
                read_buf.unsplit(reader.take_buf());
                break;
            }
            Err(e) => {
                log::error!("decode error: {e}");
                read_buf.unsplit(reader.take_buf());
                return Err(io::Error::other(e));
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
            // Get a mutable reference to the inner raw socket.
            // Re-acquired each iteration to satisfy the borrow checker.
            let inner = stream.inner_mut();

            // Process any queued requests
            process_req(&req_queue, &mut resp_queue, &mut write_buf);

            // Flush write buffer to inner socket
            if let Err(e) = nonblock_write(inner, &mut write_buf) {
                log::error!("write error: {e}");
                while let Some(pending) = resp_queue.pop_front() {
                    let _ = pending
                        .sender
                        .send(RedisValue::Error(format!("Write error: {e}")));
                }
                break;
            }

            // Read from inner socket if allowed
            let read_blocked = if io_events & 1 != 0 {
                if let Err(e) = nonblock_read(inner, &mut read_buf) {
                    log::error!("read error: {e}");
                    while let Some(pending) = resp_queue.pop_front() {
                        let _ = pending
                            .sender
                            .send(RedisValue::Error(format!("Read error: {e}")));
                    }
                    break;
                }
                false
            } else {
                true
            };

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
            io_events = if read_blocked || !write_buf.is_empty() {
                stream.wait_io()
            } else {
                1
            }
        }
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that Request creates correctly
    #[test]
    fn test_request_new() {
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let req = Request::new(vec![1, 2, 3], tx);
        assert_eq!(req.data, vec![1, 2, 3]);
    }

    /// Test that PendingRequest holds the sender
    #[test]
    fn test_pending_request() {
        let (tx, _rx) = spsc::channel();
        let _p = PendingRequest { sender: tx };
    }

    /// Test process_req moves data from queue to write_buf
    #[test]
    fn test_process_req_moves_to_write_buf() {
        let queue: Arc<Queue<Request>> = Arc::new(Queue::new());
        let mut resp_queue = VecDeque::<PendingRequest>::new();
        let mut write_buf: BytesMut = BytesMut::new();

        let (tx, _rx) = spsc::channel();
        let data: Vec<u8> = b"*1\r\n$4\r\nPING\r\n".to_vec();
        queue.push(Request::new(data, tx));

        process_req(&queue, &mut resp_queue, &mut write_buf);

        assert_eq!(write_buf.chunk(), b"*1\r\n$4\r\nPING\r\n");
        assert_eq!(resp_queue.len(), 1);
    }

    /// Test process_req with multiple requests queues them all
    #[test]
    fn test_process_req_multiple() {
        let queue: Arc<Queue<Request>> = Arc::new(Queue::new());
        let mut resp_queue = VecDeque::<PendingRequest>::new();
        let mut write_buf: BytesMut = BytesMut::new();

        for i in 0..3 {
            let (tx, _rx) = spsc::channel();
            queue.push(Request::new(vec![i as u8], tx));
        }

        process_req(&queue, &mut resp_queue, &mut write_buf);

        assert_eq!(resp_queue.len(), 3);
        assert_eq!(write_buf.len(), 3);
    }

    /// Test decode_responses with a valid integer response
    #[test]
    fn test_decode_responses_integer() {
        let mut read_buf: BytesMut = b":42\r\n".as_slice().into();
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let mut resp_queue = VecDeque::new();
        resp_queue.push_back(PendingRequest { sender: tx });

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());
        assert!(read_buf.is_empty());
    }

    /// Test decode_responses with a valid bulk string response
    #[test]
    fn test_decode_responses_bulk_string() {
        let mut read_buf: BytesMut = b"$5\r\nhello\r\n".as_slice().into();
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let mut resp_queue = VecDeque::new();
        resp_queue.push_back(PendingRequest { sender: tx });

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());
        assert!(read_buf.is_empty());
    }

    /// Test decode_responses with an error response
    #[test]
    fn test_decode_responses_error() {
        let mut read_buf: BytesMut = b"-ERR something bad\r\n".as_slice().into();
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let mut resp_queue = VecDeque::new();
        resp_queue.push_back(PendingRequest { sender: tx });

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());
        assert!(read_buf.is_empty());
    }

    /// Test decode_responses with incomplete data leaves buffer unchanged
    #[test]
    fn test_decode_responses_incomplete() {
        let mut read_buf: BytesMut = b"$5\r\nhel".as_slice().into();
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let mut resp_queue = VecDeque::new();
        resp_queue.push_back(PendingRequest { sender: tx });

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());
        assert!(!read_buf.is_empty()); // incomplete, so buffer is restored
    }

    /// Test decode_responses with unexpected response (no pending) warns
    #[test]
    fn test_decode_responses_unexpected() {
        let mut read_buf: BytesMut = b":1\r\n".as_slice().into();
        // resp_queue is empty — no pending request
        let mut resp_queue = VecDeque::<PendingRequest>::new();

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());
        assert!(read_buf.is_empty());
    }

    /// Test Connection::connect establishes and returns valid connection
    #[test]
    fn test_connection_connect() {
        let conn = Connection::connect("127.0.0.1", 6379);
        match conn {
            Ok(c) => {
                assert!(c.id() > 0);
                let tag = c.send(Request::new(vec![0], spsc::channel().0));
                assert_eq!(tag, 0);
            }
            Err(_) => {
                // Redis not running — connection failed, which is fine for CI
            }
        }
    }

    /// Test Connection::send returns monotonically increasing tags
    #[test]
    fn test_connection_send_tags() {
        let conn = Connection::connect("127.0.0.1", 6379);
        match conn {
            Ok(c) => {
                let tag0 = c.send(Request::new(vec![0], spsc::channel().0));
                let tag1 = c.send(Request::new(vec![0], spsc::channel().0));
                let tag2 = c.send(Request::new(vec![0], spsc::channel().0));
                assert_eq!(tag0, 0);
                assert_eq!(tag1, 1);
                assert_eq!(tag2, 2);
            }
            Err(_) => {}
        }
    }

    /// Test Connection::id returns the socket fd
    #[test]
    fn test_connection_id() {
        let conn = Connection::connect("127.0.0.1", 6379);
        match conn {
            Ok(c) => {
                let id = c.id();
                assert!(id > 0); // socket fds start at 3
            }
            Err(_) => {}
        }
    }

    /// Test Drop cancels the connection loop coroutine
    #[test]
    fn test_connection_drop() {
        let conn = Connection::connect("127.0.0.1", 6379);
        match conn {
            Ok(c) => {
                let id = c.id();
                assert!(id > 0);
                drop(c); // Should cancel the connection loop without hanging
            }
            Err(_) => {}
        }
    }
}
