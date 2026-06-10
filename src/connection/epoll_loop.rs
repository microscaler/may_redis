//! Epoll-based connection loop for may-redis.
//!
//! Spawns a single `go!` coroutine running an epoll loop that:
//! - Receives commands from application coroutines via an mpsc request queue
//! - Reads/writes the TCP socket
//! - Dispatches responses back via spsc channels using a monotonically increasing tag
//!
//! See `llmwiki/topics/connection-loop-pitfalls.md` before modifying this file.

use bytes::BytesMut;
use may::coroutine::JoinHandle;
use may::go;
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::connection::PendingRequest;
use super::connection::Request;
use super::dispatch::{
    decode_responses, error_dispatch, fail_queued_requests, process_req,
};
use super::io_read::drain_nonblock_read;
use super::io_write::nonblock_write;
use super::pubsub::PubSubMessage;
use super::StreamHandle;
use crate::core::RedisValue;
use std::sync::atomic::AtomicBool;

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
/// `JoinHandle` becomes joinable). On exit the loop:
///
/// 1. Stores `false` into `alive` so `Connection::send` fails fast
///    instead of queueing requests that can never be processed.
/// 2. Drains `resp_queue` (requests already on the wire) with
///    [`RedisValue::Error`].
/// 3. Drains the mpsc `req_queue` (requests queued but never picked
///    up by `process_req`) with [`RedisValue::Error`] — otherwise
///    those callers would block on `rx.recv()` forever.
#[allow(clippy::too_many_lines)]
pub(super) fn spawn_connection_loop(
    mut stream: super::connection_stream::ConnectionStream,
    req_queue: Arc<Queue<Request>>,
    pending_count: Arc<AtomicUsize>,
    push_sender: Option<Arc<spsc::Sender<PubSubMessage>>>,
    alive: Arc<AtomicBool>,
) -> JoinHandle<()> {
    go!(move || {
        let mut read_buf = BytesMut::with_capacity(65536);
        let mut write_buf = BytesMut::with_capacity(65536);
        let mut resp_queue = VecDeque::<PendingRequest>::with_capacity(512);
        let mut io_events = 1;

        loop {
            // (1) Drain new requests onto write_buf / resp_queue.
            process_req(&req_queue, &mut resp_queue, &mut write_buf);

            // (2) Best-effort flush of pending bytes to the socket.
            // (3) Read from the socket iff epoll said it was readable.
            //
            // Use `io_target()` — NOT `may::net::TcpStream` directly. The may
            // wrapper's `Read`/`Write` yield on EAGAIN instead of returning
            // `WouldBlock`, which breaks the epoll loop (Bug 1 in
            // `llmwiki/topics/connection-loop-pitfalls.md`). Plain TCP must
            // use `std::net::TcpStream` like `may_postgres`.
            let read_blocked = match stream.io_target() {
                super::connection_stream::IoTarget::Sys(sys) => {
                    if let Err(e) = nonblock_write(sys, &mut write_buf) {
                        log::error!("write error: {e}");
                        error_dispatch(
                            &mut resp_queue,
                            &pending_count,
                            &format!("Write error: {e}"),
                        );
                        break;
                    }
                    if io_events & 1 != 0 {
                        match drain_nonblock_read(sys, &mut read_buf) {
                            Ok(blocked) => blocked,
                            Err(e) => {
                                log::error!("read error: {e}");
                                error_dispatch(
                                    &mut resp_queue,
                                    &pending_count,
                                    &format!("Read error: {e}"),
                                );
                                break;
                            }
                        }
                    } else {
                        true
                    }
                }
                #[cfg(feature = "tls")]
                super::connection_stream::IoTarget::Tls(tls) => {
                    if let Err(e) = nonblock_write(tls, &mut write_buf) {
                        log::error!("write error: {e}");
                        error_dispatch(
                            &mut resp_queue,
                            &pending_count,
                            &format!("Write error: {e}"),
                        );
                        break;
                    }
                    if let Err(e) = tls.drive_io() {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            log::error!("tls io error: {e}");
                            error_dispatch(
                                &mut resp_queue,
                                &pending_count,
                                &format!("TLS I/O error: {e}"),
                            );
                            break;
                        }
                    }
                    if io_events & 1 != 0 {
                        match drain_nonblock_read(tls, &mut read_buf) {
                            Ok(blocked) => {
                                if let Err(e) = tls.drive_io() {
                                    if e.kind() != std::io::ErrorKind::WouldBlock {
                                        log::error!("tls io error: {e}");
                                        error_dispatch(
                                            &mut resp_queue,
                                            &pending_count,
                                            &format!("TLS I/O error: {e}"),
                                        );
                                        break;
                                    }
                                }
                                blocked
                            }
                            Err(e) => {
                                log::error!("read error: {e}");
                                error_dispatch(
                                    &mut resp_queue,
                                    &pending_count,
                                    &format!("Read error: {e}"),
                                );
                                break;
                            }
                        }
                    } else {
                        true
                    }
                }
            };

            // (4) Dispatch every full RESP value sitting in read_buf.
            //     This MAY dispatch more than one PendingRequest per
            //     call (Bug 2 — see `decode_responses` docs).
            if let Err(e) = decode_responses(
                &mut read_buf,
                &mut resp_queue,
                &pending_count,
                push_sender.as_ref().map(std::sync::Arc::as_ref),
            ) {
                log::error!("decode error: {e}");
                error_dispatch(
                    &mut resp_queue,
                    &pending_count,
                    &format!("Decode error: {e}"),
                );
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

        // On loop exit (fatal error): mark the connection dead FIRST so
        // new send() calls fail fast, then resolve everything in flight.
        alive.store(false, Ordering::SeqCst);
        while let Some(pending) = resp_queue.pop_front() {
            let _ = pending
                .sender
                .send(RedisValue::Error("Connection loop terminated".into()));
            pending_count.fetch_sub(1, Ordering::SeqCst);
        }
        fail_queued_requests(&req_queue, &pending_count, "Connection loop terminated");
    })
}
