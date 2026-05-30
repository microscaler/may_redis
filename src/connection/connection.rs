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

use bytes::BytesMut;
use may::coroutine::JoinHandle;
use may::io::{WaitIo, WaitIoWaker};
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::epoll_loop::spawn_connection_loop;
use super::connection_limits::{
    ConnectionLimitError, DEFAULT_MAX_QUEUE_DEPTH, DEFAULT_MAX_REQUEST_SIZE,
};
use super::tcp::{self, ConnectionError, TcpConnector};
use crate::core::RedisValue;

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// A request to be sent to the Redis server.
///
/// Carries the fully-encoded RESP command bytes and the
/// `may::sync::spsc::Sender` half of a one-shot response channel. The
/// matching `may::sync::spsc::Receiver` stays with the caller so it
/// can block on `recv()` until the connection loop dispatches the
/// decoded [`RedisValue`].
pub struct Request {
    pub data: Vec<u8>,
    pub sender: spsc::Sender<RedisValue>,
}

impl Request {
    /// Create a new request with the given data and channel sender.
    #[must_use]
    pub const fn new(data: Vec<u8>, sender: spsc::Sender<RedisValue>) -> Self {
        Self { data, sender }
    }
}

// ---------------------------------------------------------------------------
// PendingRequest
// ---------------------------------------------------------------------------

/// Internal state tracked per pending request for response dispatch.
pub(super) struct PendingRequest {
    pub(super) sender: spsc::Sender<RedisValue>,
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// A live connection to a Redis server, owning the background loop coroutine.
///
/// Dropping the `Connection` cancels the loop coroutine via
/// `may::coroutine::Coroutine::cancel`; any pending requests still
/// in flight will have their `spsc::Sender` dropped and the caller's
/// `rx.recv()` will return an error.
pub struct Connection {
    io_handle: JoinHandle<()>,
    req_queue: Arc<Queue<Request>>,
    waker: WaitIoWaker,
    id: usize,
    tag_counter: Arc<AtomicUsize>,
    max_queue_depth: usize,
    max_request_size: usize,
    pending_count: Arc<AtomicUsize>,
    ssrf_config: Option<tcp::SsrfConfig>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let rx = self.io_handle.coroutine();
        // SAFETY: May's coroutine cancellation guarantees the target coroutine will
        // stop execution at its next yield point.
        unsafe { rx.cancel() };
    }
}

impl Connection {
    /// Establish a TCP connection to the Redis server and spawn the
    /// background connection loop coroutine.
    pub fn connect(host: &str, port: u16) -> Result<Self, ConnectionError> {
        let stream = TcpConnector::connect(host, port)?;
        let id = stream.as_raw_fd() as usize;
        let waker = stream.waker();
        let req_queue = Arc::new(Queue::new());
        let pending_count = Arc::new(AtomicUsize::new(0));
        let io_handle = spawn_connection_loop(stream, req_queue.clone(), pending_count.clone());
        Ok(Self {
            io_handle,
            req_queue,
            waker,
            id,
            tag_counter: Arc::new(AtomicUsize::new(0)),
            max_queue_depth: DEFAULT_MAX_QUEUE_DEPTH,
            max_request_size: DEFAULT_MAX_REQUEST_SIZE,
            pending_count,
            ssrf_config: None,
        })
    }

    /// Establish a TCP connection with SSRF protection enabled.
    pub fn connect_with_ssrf_protection(
        host: &str,
        port: u16,
        timeout: std::time::Duration,
        ssrf_config: tcp::SsrfConfig,
    ) -> Result<Self, ConnectionError> {
        let stream = TcpConnector::connect_with_ssrf_check(host, port, timeout, ssrf_config)?;
        let id = stream.as_raw_fd() as usize;
        let waker = stream.waker();
        let req_queue = Arc::new(Queue::new());
        let pending_count = Arc::new(AtomicUsize::new(0));
        let io_handle = spawn_connection_loop(stream, req_queue.clone(), pending_count.clone());
        Ok(Self {
            io_handle,
            req_queue,
            waker,
            id,
            tag_counter: Arc::new(AtomicUsize::new(0)),
            max_queue_depth: DEFAULT_MAX_QUEUE_DEPTH,
            max_request_size: DEFAULT_MAX_REQUEST_SIZE,
            pending_count,
            ssrf_config: Some(ssrf_config),
        })
    }

    /// Establish a TCP connection with configurable resource limits.
    pub fn connect_with_limits(
        host: &str,
        port: u16,
        timeout: std::time::Duration,
        max_queue_depth: usize,
        max_request_size: usize,
    ) -> Result<Self, ConnectionError> {
        let stream = TcpConnector::connect_with_timeout(host, port, timeout)?;
        let id = stream.as_raw_fd() as usize;
        let waker = stream.waker();
        let req_queue = Arc::new(Queue::new());
        let pending_count = Arc::new(AtomicUsize::new(0));
        let io_handle = spawn_connection_loop(stream, req_queue.clone(), pending_count.clone());
        Ok(Self {
            io_handle,
            req_queue,
            waker,
            id,
            tag_counter: Arc::new(AtomicUsize::new(0)),
            max_queue_depth,
            max_request_size,
            pending_count,
            ssrf_config: None,
        })
    }

    #[must_use]
    pub const fn ssrf_config(&self) -> Option<&tcp::SsrfConfig> {
        self.ssrf_config.as_ref()
    }

    #[must_use]
    pub fn send(&self, request: Request) -> Result<usize, ConnectionLimitError> {
        if self.pending_count.load(Ordering::SeqCst) >= self.max_queue_depth {
            return Err(ConnectionLimitError::QueueFull(self.max_queue_depth));
        }
        if request.data.len() > self.max_request_size {
            return Err(ConnectionLimitError::RequestTooLarge(
                self.max_request_size,
                request.data.len(),
            ));
        }
        let tag = self.tag_counter.fetch_add(1, Ordering::SeqCst);
        self.pending_count.fetch_add(1, Ordering::SeqCst);
        self.req_queue.push(request);
        self.waker.wakeup();
        Ok(tag)
    }

    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }
}
