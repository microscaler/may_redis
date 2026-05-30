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

use bytes::{BufMut, BytesMut};
use may::coroutine::JoinHandle;
use may::io::{WaitIo, WaitIoWaker};
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::connection_io::spawn_connection_loop;
use super::tcp::{self, ConnectionError, TcpConnector};
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
pub(super) struct PendingRequest {
    pub(super) sender: spsc::Sender<RedisValue>,
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
    /// Maximum number of pending requests in the queue (Story 3, Issue #7).
    max_queue_depth: usize,
    /// Maximum size in bytes for a single request's RESP data (Story 3, Issue #7).
    max_request_size: usize,
    /// Current pending request count (for O(1) bounded queue check, NFR-012).
    pending_count: Arc<AtomicUsize>,
    /// SSRF configuration — if Some, resolved IPs are checked against deny-lists.
    /// Story 3, Issue #8: SSRF protection for DNS-resolved connections.
    ssrf_config: Option<tcp::SsrfConfig>,
}

/// Connection errors for resource limit violations (Story 3, Issue #7).
#[derive(Debug)]
pub enum ConnectionLimitError {
    /// Request queue is full.
    QueueFull(usize),
    /// Request exceeds the maximum size.
    RequestTooLarge(usize, usize),
}

impl std::fmt::Display for ConnectionLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull(max) => {
                write!(f, "request queue is full (max {max} pending requests)")
            }
            Self::RequestTooLarge(max, got) => {
                write!(f, "request too large (max {max} bytes, got {got})")
            }
        }
    }
}

impl std::error::Error for ConnectionLimitError {}

/// Default limits for a safe connection (Story 3, Issue #7, AC-3.1, AC-3.4).
const DEFAULT_MAX_QUEUE_DEPTH: usize = 1024;
const DEFAULT_MAX_REQUEST_SIZE: usize = 65536; // 64 KiB

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
    ///
    /// After DNS resolution, every resolved IP is checked against the
    /// deny-list in `ssrf_config`. If ANY resolved address matches,
    /// connection is refused.
    ///
    /// FR-027: New constructor that enables SSRF checks.
    /// AC-3.9: This constructor enables SSRF protection by default.
    ///
    /// # Arguments
    /// * `host` — Server hostname or IP address
    /// * `port` — Server port
    /// * `timeout` — Maximum duration to wait for the connection
    /// * `ssrf_config` — Configuration for which IP ranges to block
    ///
    /// # Errors
    /// Returns [`ConnectionError::SsrfViolation`] if any resolved address
    /// is in a deny-listed range, otherwise same as [`Self::connect`].
    ///
    /// # Coroutine context
    ///
    /// MUST be called from inside a may coroutine context (see [`Self::connect`]).
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
    ///
    /// FR-023: New constructor that accepts custom queue depth and request
    /// size limits for Issue #7 (Unbounded Request Queue).
    ///
    /// # Arguments
    /// * `host` — Server hostname or IP address
    /// * `port` — Server port
    /// * `timeout` — Maximum duration to wait for the connection
    /// * `max_queue_depth` — Maximum pending requests (default: 1024)
    /// * `max_request_size` — Maximum request size in bytes (default: 64 KiB)
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if DNS resolution or TCP connect fails.
    ///
    /// # Coroutine context
    ///
    /// MUST be called from inside a may coroutine context (see [`Self::connect`]).
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

    /// Returns the SSRF configuration for this connection, if SSRF protection
    /// is enabled.
    #[must_use = "returns the SSRF configuration if enabled"]
    pub const fn ssrf_config(&self) -> Option<&tcp::SsrfConfig> {
        self.ssrf_config.as_ref()
    }

    /// Enqueue `request` for the connection loop.
    ///
    /// Atomically:
    ///
    /// 1. checks resource limits (Story 3, Issue #7): queue depth and request size;
    ///    returns [`ConnectionLimitError`] if a limit is exceeded;
    /// 2. assigns a monotonic tag (returned to the caller — useful for
    ///    correlating logs / metrics but **not** used for response
    ///    matching, which is purely positional);
    /// 3. pushes the [`Request`] (including its `spsc::Sender`) onto
    ///    the shared mpsc queue;
    /// 4. signals the connection loop's [`WaitIoWaker`] so any
    ///    in-flight `stream.wait_io()` returns immediately and the
    ///    freshly-queued request is processed on the very next
    ///    iteration instead of waiting for socket I/O readiness.
    ///
    /// # Returns
    ///
    /// `Ok(tag)` on success. `Err(ConnectionLimitError)` if the queue is
    /// full or the request exceeds the maximum size.
    ///
    /// This method is non-blocking: it does NOT wait for the request
    /// to be written or for the response to come back. The caller is
    /// expected to keep the matching `may::sync::spsc::Receiver` and
    /// call `recv()` on it to obtain the [`RedisValue`] response.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionLimitError::QueueFull`] if the pending request
    /// queue has reached its maximum depth.
    ///
    /// Returns [`ConnectionLimitError::RequestTooLarge`] if the request
    /// data exceeds the configured maximum size.
    #[must_use = "returns the tag assigned to the enqueued request"]
    pub fn send(&self, request: Request) -> Result<usize, ConnectionLimitError> {
        // Story 3, Issue #7: enforce resource limits before sending (AC-3.1–AC-3.4)
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

    /// Returns the unique connection identifier (socket fd).
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }
}
