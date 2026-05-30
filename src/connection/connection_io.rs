//! Connection I/O helpers and the epoll-based connection loop.
//!
//! This module contains:
//! - `process_req` — drain the mpsc request queue into write/response buffers
//! - `nonblock_read` — non-blocking read from the TCP socket
//! - `nonblock_write` — non-blocking write to the TCP socket
//! - `decode_responses` — decode RESP values from the read buffer
//! - `spawn_connection_loop` — the epoll loop running in a may coroutine

use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use may::coroutine::JoinHandle;
use may::go;
use may::io::WaitIo;
use may::net::TcpStream;
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::collections::VecDeque;
use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::codec::reader::RESPReader;
use crate::core::{RedisError, RedisValue};

use super::connection::PendingRequest;
use super::connection::Request;

const MIN_BUFFER_CAPACITY: usize = 512;

const WRITE_BUF_RESERVE_TARGET: usize = 65536;

pub(super) fn process_req(
    queue: &Queue<Request>,
    resp_queue: &mut VecDeque<PendingRequest>,
    write_buf: &mut BytesMut,
) {
    while let Some(req) = queue.pop() {
        let rem = write_buf.capacity() - write_buf.len();
        if rem < MIN_BUFFER_CAPACITY {
            write_buf.reserve(WRITE_BUF_RESERVE_TARGET - rem);
        }
        resp_queue.push_back(PendingRequest { sender: req.sender });
        write_buf.put_slice(&req.data);
    }
}

/// Decrement the pending request counter after a response is dispatched
/// or an error forces a request out of the pipeline.
fn release_pending(pending_count: &Arc<AtomicUsize>) {
    pending_count.fetch_sub(1, Ordering::SeqCst);
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
fn nonblock_read<R: io::Read>(stream: &mut R, read_buf: &mut BytesMut) -> io::Result<bool> {
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
fn nonblock_write<W: io::Write>(stream: &mut W, write_buf: &mut BytesMut) -> io::Result<usize> {
    let buf = write_buf.chunk();
    let len = buf.len();
    let mut write_cnt = 0;
    while write_cnt < len {
        // AC-4.1, FR-033: Use safe bounds-checked indexing instead of `get_unchecked`.
        // The `write_cnt <= len` loop invariant (checked by `while` condition) guarantees
        // this slice is always valid. `buf[write_cnt..]` returns a subslice of `buf`
        // that is at most `buf.len() - write_cnt` bytes — never past the end.
        match stream.write(&buf[write_cnt..]) {
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
pub(super) fn decode_responses(
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
pub(super) fn spawn_connection_loop(
    mut stream: TcpStream,
    req_queue: Arc<Queue<Request>>,
    pending_count: Arc<AtomicUsize>,
) -> JoinHandle<()> {
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
                    release_pending(&pending_count);
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
                            release_pending(&pending_count);
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
                    release_pending(&pending_count);
                }
                break;
            }

            // Decrement pending count for each dispatched response.
            while resp_queue.pop_front().is_some() {
                release_pending(&pending_count);
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

        // On loop exit (fatal error), drain remaining pending requests.
        while let Some(pending) = resp_queue.pop_front() {
            let _ = pending
                .sender
                .send(RedisValue::Error("Connection loop terminated".into()));
            release_pending(&pending_count);
        }
    })
}
