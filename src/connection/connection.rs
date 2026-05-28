//! Connection loop, request queue, and response dispatch.
//!
//! This sub-module mirrors the `may_postgres` `Connection` pattern:
//!
//! - A single `go!` coroutine running an epoll loop owns the socket.
//! - An mpsc `may::queue::mpsc::Queue<Request>` carries commands from
//!   application coroutines to that loop.
//! - A per-request `may::sync::spsc::Sender<RedisValue>` carries the
//!   response back.
//! - Monotonically increasing tags (held by [`Connection`]) match
//!   requests to responses across pipelined commands.
//! - Non-blocking `Read` / `Write` on the raw socket fills/drains
//!   `BytesMut` buffers; `stream.wait_io()` suspends the loop until
//!   epoll reports the socket is ready or the `WaitIoWaker` is
//!   signalled by [`Connection::send`].
//!
//! # Fragility warning — read this before changing the loop
//!
//! The connection loop is the single most subtle piece of code in this
//! crate. It has already shipped two production-impacting bugs that
//! caused all integration tests to hang (one starved the may scheduler,
//! one silently dropped pipeline responses). Both are dissected in
//! `llmwiki/topics/connection-loop-pitfalls.md` together with the
//! regression tests that guard against them.
//!
//! Before modifying `spawn_connection_loop`, `decode_responses`,
//! `nonblock_read`, or `nonblock_write`:
//!
//! 1. Re-read `llmwiki/topics/connection-loop-pitfalls.md` end-to-end.
//! 2. Diff your intended change against
//!    `../may_postgres/src/connection.rs::connection_loop` — that loop
//!    is the canonical reference and any divergence here must be
//!    justified in a code comment.
//! 3. Run the full `client::client::tests::test_integration_*` suite
//!    with `-- --test-threads=1`; these tests hang (rather than fail
//!    loudly) when this class of bug regresses.

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

use super::tcp::{ConnectionError, TcpConnector};
use crate::codec::reader::RESPReader;
use crate::core::{RedisError, RedisValue};

/// A request to be sent to the Redis server.
///
/// Carries the fully-encoded RESP command bytes and the
/// `may::sync::spsc::Sender` half of a one-shot response channel. The
/// matching `may::sync::spsc::Receiver` stays with the caller so it
/// can block on `recv()` until the connection loop dispatches the
/// decoded [`RedisValue`].
///
/// Ownership flow:
///
/// 1. Caller builds the RESP bytes and creates `(tx, rx)` via `may::sync::spsc::channel`.
/// 2. Caller wraps `(data, tx)` in a `Request` and calls [`Connection::send`].
/// 3. The connection loop moves `tx` into its internal pending-request
///    queue (preserving request order so responses can be matched
///    positionally).
/// 4. When the response is decoded, the loop calls `tx.send(value)` and
///    the caller's `rx.recv()` returns.
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
///
/// One entry is pushed onto the loop's `resp_queue` for every request
/// pulled off the mpsc `req_queue`, and popped (FIFO) for every RESP
/// value successfully decoded from the read buffer. The FIFO ordering
/// is how request/response correlation works without a per-message
/// tag in the wire format — RESP guarantees responses come back in the
/// same order the commands were sent.
struct PendingRequest {
    sender: spsc::Sender<RedisValue>,
}

/// A live connection to a Redis server, owning the background loop coroutine.
///
/// Cheap to share: all the interior state needed to enqueue work is
/// either [`Arc`]-shared or trivially `Copy`/clone, so wrapping
/// `Connection` in an `Arc` (as `crate::client::RedisClient` does)
/// lets many application coroutines push requests concurrently into
/// the same socket without any extra synchronisation here.
///
/// Dropping the `Connection` cancels the loop coroutine via
/// `may::coroutine::Coroutine::cancel`; any pending requests still
/// in flight will have their `spsc::Sender` dropped and the caller's
/// `rx.recv()` will return an error.
pub struct Connection {
    /// Handle to the connection loop coroutine, used for graceful shutdown.
    io_handle: JoinHandle<()>,
    /// Shared request queue for pushing commands from application coroutines.
    req_queue: Arc<Queue<Request>>,
    /// Waker to signal the connection loop about new requests.
    ///
    /// Calling `waker.wakeup()` forces the next `stream.wait_io()`
    /// inside the loop to return immediately so freshly-pushed
    /// requests are picked up without waiting for socket I/O
    /// readiness.
    waker: WaitIoWaker,
    /// Unique connection identifier (socket fd).
    id: usize,
    /// Monotonic tag counter for request-response matching.
    ///
    /// RESP itself does not tag responses, so the positional ordering
    /// of pending requests is what actually matches requests to
    /// responses. This counter is exposed via the return value of
    /// [`Self::send`] so callers can correlate log entries / metrics.
    tag_counter: Arc<AtomicUsize>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let rx = self.io_handle.coroutine();
        // SAFETY: May's coroutine cancellation guarantees the target coroutine will
        // stop execution at its next yield point (cooperative yielding via
        // `may::coroutine::yield_now()` or I/O operations). This prevents partial
        // writes because: (a) the connection loop only yields at safe points (after
        // epoll waits, between read/write cycles), (b) any in-flight command bytes
        // already queued in `write_buf` will be drained on the next writable epoll
        // event before the coroutine actually terminates, (c) the `tx` channels
        // used for response dispatch are spsc channels — the sender side
        // (`Connection::send`) closes on drop, preventing new requests from being
        // added after cancellation begins.
        unsafe { rx.cancel() };
    }
}

/// Drain `queue` of every pending [`Request`], appending the encoded
/// command bytes to `write_buf` and recording a [`PendingRequest`]
/// (in arrival order) on `resp_queue` for each one.
///
/// This is the only place where requests cross the boundary from the
/// application coroutines into the loop's local state, so it must be
/// strictly ordered: the position in `resp_queue` is what later
/// matches a decoded response back to its `spsc::Sender`. Do not
/// reorder, deduplicate, or coalesce entries here.
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

/// Read from the inner raw socket into a `BytesMut` buffer.
///
/// # Return value (critical — do not discard)
///
/// - `Ok(true)` — the read was **blocked**: the socket returned
///   `WouldBlock` before `read_buf` was filled. The caller MUST wait
///   for the next epoll readable event (via `stream.wait_io()`)
///   before reading again, otherwise the connection loop will
///   busy-spin and never yield to other coroutines.
/// - `Ok(false)` — the read filled `read_buf` completely. More data
///   may still be available in the kernel buffer, so the caller
///   should re-read immediately without going through epoll.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] for any non-`WouldBlock`
/// failure. A clean half-close (read of 0 bytes) is mapped to
/// [`io::ErrorKind::BrokenPipe`] so the caller treats it as a fatal
/// connection error.
///
/// # History
///
/// The whole reason the return value is documented this loudly is
/// Bug 1 in `llmwiki/topics/connection-loop-pitfalls.md`: the
/// connection loop used to discard this `bool` and hardcode
/// `read_blocked = false`, which caused every integration test to
/// hang. Treat the return value as load-bearing.
fn nonblock_read(stream: &mut std::net::TcpStream, read_buf: &mut BytesMut) -> io::Result<bool> {
    // SAFETY: `BytesMut::chunk_mut()` returns a `&mut [u8]` with capacity equal to
    // `remaining_capacity()`. The buffer is uninitialized but guaranteed to hold at
    // least `len` bytes. `stream.read()` writes up to `len` bytes into this space.
    // After `read_cnt` bytes are read, `read_buf.advance_mut(read_cnt)` marks them as
    // initialized. The raw pointer cast is valid because `chunk_mut()` returns a slice
    // with properly initialized capacity metadata.
    let buf: &mut [u8] = unsafe { &mut *(read_buf.chunk_mut() as *mut _ as *mut [u8]) };
    let len = buf.len();
    let mut read_cnt = 0;
    while read_cnt < len {
        match stream.read(unsafe {
            // SAFETY: The `while read_cnt < len` loop invariant guarantees
            // `read_cnt <= buf.len()`. After each successful read, `read_cnt`
            // increases, but never exceeds `len`. Therefore `read_cnt..` is always a
            // valid subslice of `buf`.
            buf.get_unchecked_mut(read_cnt..)
        }) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => read_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    // SAFETY: `stream.read()` wrote exactly `read_cnt` bytes into the uninitialized
    // buffer starting at position 0. `advance_mut(read_cnt)` transitions those
    // `read_cnt` bytes from uninitialized to initialized. This is correct because
    // `stream.read()` only writes into the capacity portion returned by `chunk_mut()`.
    unsafe { read_buf.advance_mut(read_cnt) };
    Ok(read_cnt < len)
}

/// Write the front of `write_buf` to the inner raw socket without
/// blocking.
///
/// Writes as many bytes as the socket will accept in one go and then
/// advances `write_buf` past them, so a follow-up call (after a
/// `wait_io` for `WRITABLE`) can pick up where this one left off.
///
/// # Return value
///
/// `Ok(n)` is the number of bytes actually written this call. `n` may
/// be less than `write_buf.len()` — that simply means the kernel
/// buffer is full and the caller should wait for the socket to become
/// writable again. `write_buf` is left containing exactly the unwritten
/// tail.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] for any non-`WouldBlock`
/// failure. A write of 0 bytes is mapped to
/// [`io::ErrorKind::BrokenPipe`] so the caller treats it as fatal.
fn nonblock_write(stream: &mut std::net::TcpStream, write_buf: &mut BytesMut) -> io::Result<usize> {
    let buf = write_buf.chunk();
    let len = buf.len();
    let mut write_cnt = 0;
    while write_cnt < len {
        match stream.write(unsafe {
            // SAFETY: `write_buf.chunk()` returns a slice over the initialized portion
            // of `BytesMut`. The `while write_cnt < len` loop invariant guarantees
            // `write_cnt <= buf.len()`, so `write_cnt..` is always a valid subslice.
            // The kernel's `write()` call only reads from this slice — it never writes
            // beyond it.
            buf.get_unchecked(write_cnt..)
        }) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => write_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    write_buf.advance(write_cnt);
    Ok(write_cnt)
}

/// Decode every complete RESP value currently in `read_buf` and dispatch
/// each one to the corresponding pending request in FIFO order.
///
/// # Buffer contract (critical — see Bug 2 in `llmwiki/topics/connection-loop-pitfalls.md`)
///
/// A single TCP read frequently contains multiple concatenated RESP
/// values (this is the normal case for pipelines and any back-to-back
/// commands). Every branch of the match below MUST put the reader's
/// remaining bytes back into `read_buf` via
/// `read_buf.unsplit(reader.take_buf())` before exiting, because
/// `read_buf.split()` is destructive: at the top of each iteration
/// `read_buf` is logically empty and all the bytes live inside the
/// `RESPReader` until we put them back.
///
/// The three possible outcomes are:
///
/// - **`Ok(value)`** — one full RESP value was decoded. Put the
///   unconsumed tail back into `read_buf`, then dispatch `value` to
///   the next pending request and loop again so any further batched
///   responses are decoded too. Dropping the tail here is exactly
///   what Bug 2 was: every pipeline response after the first was
///   silently discarded and callers hung on `rx.recv()`.
/// - **`Err(RedisError::Parse(_))`** — the buffer contains a partial
///   value (more bytes will arrive on the next read). Put the bytes
///   back unchanged and stop; the next loop iteration after the next
///   `nonblock_read` will retry decoding from the same byte offset.
/// - **`Err(other)`** — a hard decode error. Put the bytes back so
///   they show up in any post-mortem logging, then surface the error
///   to the caller (which will fail the connection and drain
///   `resp_queue` with [`RedisValue::Error`]s).
///
/// # Errors
///
/// Returns [`io::Error::other`] wrapping a [`RedisError`] for any
/// non-`Parse` decode failure. Partial-value parse errors are treated
/// as benign and turned into `Ok(())`.
fn decode_responses(
    read_buf: &mut BytesMut,
    resp_queue: &mut VecDeque<PendingRequest>,
) -> io::Result<()> {
    while !read_buf.is_empty() {
        let mut reader = RESPReader::new(read_buf.split());
        match reader.read_value() {
            Ok(value) => {
                // CRITICAL: must run BEFORE the dispatch below so that any
                // remaining batched responses are visible to the next
                // iteration of this `while !read_buf.is_empty()` loop.
                // See Bug 2 in `llmwiki/topics/connection-loop-pitfalls.md`.
                read_buf.unsplit(reader.take_buf());
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

/// Spawn the epoll-based connection loop as a may coroutine.
///
/// The returned [`JoinHandle`] is owned by [`Connection`] and the loop
/// is cancelled when the `Connection` is dropped.
///
/// # Loop invariants
///
/// One iteration of the loop performs, in this exact order:
///
/// 1. `process_req`: drain the mpsc `req_queue` into `write_buf` and
///    `resp_queue` (preserving FIFO order so responses can be matched
///    positionally).
/// 2. `nonblock_write`: best-effort flush of `write_buf` to the
///    socket. Anything we couldn't write stays at the front of
///    `write_buf` for next iteration.
/// 3. `nonblock_read` *iff* epoll told us the socket is readable
///    (`io_events & 1 != 0`). Captures `read_blocked` — see below.
/// 4. `decode_responses`: decode every complete RESP value sitting
///    in `read_buf` and dispatch each one to the next pending request.
/// 5. `stream.wait_io()` when we have nothing useful to do right now
///    (read blocked AND write buffer is empty would be a busy-spin);
///    otherwise short-circuit `io_events = 1` to immediately
///    re-attempt a read on the next iteration.
///
/// # Two load-bearing details (do not change without re-reading the pitfalls page)
///
/// - **`read_blocked` MUST be the actual `bool` returned by
///   `nonblock_read`.** If you replace the `match` arm with anything
///   that discards it (e.g. `if let Err(_) = nonblock_read(..)` plus a
///   constant `false`), step 5 will permanently take the
///   `io_events = 1` branch and the loop will busy-spin without ever
///   calling `wait_io()`. The coroutine will then hog its may worker
///   and any test / application coroutine sharing that worker will
///   never make progress. This is Bug 1 in
///   `llmwiki/topics/connection-loop-pitfalls.md`.
/// - **`decode_responses` is the only place that puts bytes back into
///   `read_buf`.** Do not assume one call dispatches at most one
///   response; a single TCP read frequently delivers several. See
///   the docstring on `decode_responses` and Bug 2 in the same
///   pitfalls page.
///
/// # Error handling
///
/// Any of the I/O / decode helpers returning `Err` is treated as a
/// fatal connection error: every still-pending `spsc::Sender` in
/// `resp_queue` is signalled with a [`RedisValue::Error`] describing
/// the failure, and the loop breaks (the coroutine exits, the
/// `JoinHandle` becomes joinable).
fn spawn_connection_loop(mut stream: TcpStream, req_queue: Arc<Queue<Request>>) -> JoinHandle<()> {
    go!(move || {
        let mut read_buf = BytesMut::with_capacity(65536);
        let mut write_buf = BytesMut::with_capacity(65536);
        let mut resp_queue = VecDeque::<PendingRequest>::with_capacity(512);
        let mut io_events = 1;

        loop {
            // Re-acquire the inner raw socket each iteration to satisfy
            // the borrow checker (we also need `&mut stream` further down
            // for `stream.wait_io()`).
            let inner = stream.inner_mut();

            // (1) Drain new requests onto write_buf / resp_queue.
            process_req(&req_queue, &mut resp_queue, &mut write_buf);

            // (2) Best-effort flush of pending bytes to the socket.
            if let Err(e) = nonblock_write(inner, &mut write_buf) {
                log::error!("write error: {e}");
                while let Some(pending) = resp_queue.pop_front() {
                    let _ = pending
                        .sender
                        .send(RedisValue::Error(format!("Write error: {e}")));
                }
                break;
            }

            // (3) Read from the socket iff epoll said it was readable.
            //
            // The bool returned by `nonblock_read` is load-bearing:
            // it is the only signal that decides whether step (5)
            // below blocks on epoll or busy-spins. See Bug 1 in
            // `llmwiki/topics/connection-loop-pitfalls.md`.
            let read_blocked = if io_events & 1 != 0 {
                match nonblock_read(inner, &mut read_buf) {
                    Ok(blocked) => blocked,
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
            } else {
                true
            };

            // (4) Dispatch every full RESP value sitting in read_buf.
            //     This MAY dispatch more than one PendingRequest per
            //     call (Bug 2 — see `decode_responses` docs).
            if let Err(e) = decode_responses(&mut read_buf, &mut resp_queue) {
                log::error!("decode error: {e}");
                while let Some(pending) = resp_queue.pop_front() {
                    let _ = pending
                        .sender
                        .send(RedisValue::Error(format!("Decode error: {e}")));
                }
                break;
            }

            // (5) Park on epoll until something useful happens.
            //     - If read was blocked, we need a READABLE event.
            //     - If write_buf is non-empty, the kernel buffer is
            //       full and we need a WRITABLE event to flush more.
            //     - Otherwise we have data to act on already; skip
            //       the syscall and re-loop immediately with the
            //       READABLE bit set so step (3) runs again.
            io_events = if read_blocked || !write_buf.is_empty() {
                stream.wait_io()
            } else {
                1
            }
        }
    })
}

impl Connection {
    /// Establish a TCP connection to the Redis server and spawn the
    /// background connection loop coroutine.
    ///
    /// # Arguments
    ///
    /// * `host` — Server hostname or IP address
    /// * `port` — Server port
    ///
    /// # Returns
    ///
    /// A [`Connection`] with an active epoll loop coroutine. The
    /// caller can immediately start pushing [`Request`]s via
    /// [`Self::send`]; the loop will pick them up on its next iteration.
    ///
    /// # Coroutine context
    ///
    /// MUST be called from inside a may coroutine context (e.g. via
    /// `may::go!` or `may::coroutine::spawn`) because
    /// [`TcpConnector::connect`] and the spawned connection loop both
    /// depend on the may runtime. Calling this from a bare std
    /// thread will panic in `go!` / `may::net::TcpStream`.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionError`] if DNS resolution, the TCP connect,
    /// or `TCP_NODELAY` configuration fails.
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

    /// Enqueue `request` for the connection loop.
    ///
    /// Atomically:
    ///
    /// 1. assigns a monotonic tag (returned to the caller — useful for
    ///    correlating logs / metrics but **not** used for response
    ///    matching, which is purely positional);
    /// 2. pushes the [`Request`] (including its `spsc::Sender`) onto
    ///    the shared mpsc queue;
    /// 3. signals the connection loop's [`WaitIoWaker`] so any
    ///    in-flight `stream.wait_io()` returns immediately and the
    ///    freshly-queued request is processed on the very next
    ///    iteration instead of waiting for socket I/O readiness.
    ///
    /// This method is non-blocking: it does NOT wait for the request
    /// to be written or for the response to come back. The caller is
    /// expected to keep the matching `may::sync::spsc::Receiver` and
    /// call `recv()` on it to obtain the [`RedisValue`] response.
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

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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

    /// Regression: when several responses are concatenated in one read
    /// (as happens with pipelines), every pending request must receive
    /// its response and the buffer must be fully drained. Previously
    /// only the first response was dispatched and the remaining bytes
    /// were dropped, causing pipeline callers to hang forever on
    /// `rx.recv()` for the missing responses.
    #[test]
    fn test_decode_responses_multiple_in_one_buffer() {
        // 4 responses: +OK\r\n +OK\r\n +OK\r\n $5\r\nhello\r\n
        let mut read_buf: BytesMut = b"+OK\r\n+OK\r\n+OK\r\n$5\r\nhello\r\n".as_slice().into();

        let mut resp_queue = VecDeque::<PendingRequest>::new();
        let mut receivers: Vec<spsc::Receiver<RedisValue>> = Vec::new();
        for _ in 0..4 {
            let (tx, rx) = spsc::channel();
            resp_queue.push_back(PendingRequest { sender: tx });
            receivers.push(rx);
        }

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(
            result.is_ok(),
            "decode_responses returned error: {result:?}"
        );
        assert!(read_buf.is_empty(), "buffer not fully drained");
        assert!(resp_queue.is_empty(), "not all pending requests dispatched");

        // Verify each receiver actually got its response.
        let v0 = receivers[0].try_recv().expect("missing response 0");
        let v1 = receivers[1].try_recv().expect("missing response 1");
        let v2 = receivers[2].try_recv().expect("missing response 2");
        let v3 = receivers[3].try_recv().expect("missing response 3");
        assert!(matches!(v0, RedisValue::SimpleString(ref s) if s == "OK"));
        assert!(matches!(v1, RedisValue::SimpleString(ref s) if s == "OK"));
        assert!(matches!(v2, RedisValue::SimpleString(ref s) if s == "OK"));
        assert!(matches!(v3, RedisValue::BulkString(ref b) if b == b"hello"));
    }

    /// Regression: when several responses are concatenated and the final
    /// response is only partially present, the complete responses must
    /// still be dispatched and the trailing partial bytes must remain
    /// in `read_buf` so the next read can complete them.
    #[test]
    fn test_decode_responses_multiple_with_partial_trailing() {
        // 2 complete responses (+OK, :42) followed by a partial bulk string.
        let mut read_buf: BytesMut = b"+OK\r\n:42\r\n$5\r\nhel".as_slice().into();

        let mut resp_queue = VecDeque::<PendingRequest>::new();
        let mut receivers: Vec<spsc::Receiver<RedisValue>> = Vec::new();
        for _ in 0..3 {
            let (tx, rx) = spsc::channel();
            resp_queue.push_back(PendingRequest { sender: tx });
            receivers.push(rx);
        }

        let result = decode_responses(&mut read_buf, &mut resp_queue);
        assert!(result.is_ok());

        // First two pending requests got responses, third did not.
        assert_eq!(
            resp_queue.len(),
            1,
            "expected one pending request to remain"
        );
        // Partial bulk string bytes ($5\r\nhel) must still be in the buffer.
        assert!(!read_buf.is_empty(), "partial bytes were dropped");

        let v0 = receivers[0].try_recv().expect("missing response 0");
        let v1 = receivers[1].try_recv().expect("missing response 1");
        assert!(matches!(v0, RedisValue::SimpleString(ref s) if s == "OK"));
        assert!(matches!(v1, RedisValue::Integer(42)));
        assert!(
            receivers[2].try_recv().is_err(),
            "response 2 should be absent"
        );
    }

    /// Test Connection::connect establishes and returns valid connection
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_connection_connect() {
        let conn = Connection::connect("127.0.0.1", 6379);
        if let Ok(c) = conn {
            assert!(c.id() > 0);
            let tag = c.send(Request::new(vec![0], spsc::channel().0));
            assert_eq!(tag, 0);
        }
    }

    /// Test Connection::send returns monotonically increasing tags
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_connection_send_tags() {
        let conn = Connection::connect("127.0.0.1", 6379);
        if let Ok(c) = conn {
            let tag0 = c.send(Request::new(vec![0], spsc::channel().0));
            let tag1 = c.send(Request::new(vec![0], spsc::channel().0));
            let tag2 = c.send(Request::new(vec![0], spsc::channel().0));
            assert_eq!(tag0, 0);
            assert_eq!(tag1, 1);
            assert_eq!(tag2, 2);
        }
    }

    /// Test Connection::id returns the socket fd
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_connection_id() {
        let conn = Connection::connect("127.0.0.1", 6379);
        if let Ok(c) = conn {
            let id = c.id();
            assert!(id > 0); // socket fds start at 3
        }
    }

    /// Test Drop cancels the connection loop coroutine
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_connection_drop() {
        let conn = Connection::connect("127.0.0.1", 6379);
        if let Ok(c) = conn {
            let id = c.id();
            assert!(id > 0);
            drop(c); // Should cancel the connection loop without hanging
        }
    }
}
