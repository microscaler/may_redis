//! Request queue draining and RESP response decoding/dispatch.
//!
//! Handles:
//! - `process_req` — drain the mpsc request queue into write/response buffers
//! - `decode_responses` — decode RESP values and dispatch to pending requests
//! - `release_pending` — decrement the pending request counter

use bytes::BufMut;
use bytes::BytesMut;
use may::queue::mpsc::Queue;
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::codec::reader::RESPReader;
use crate::core::{RedisError, RedisValue};

use super::connection::{PendingRequest, Request};
use super::pubsub::{is_pubsub_push, parse_pubsub_push};
use may::sync::spsc;

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
/// # Pub/sub pushes
///
/// `message` / `pmessage` pushes are routed to `push_sender` and are
/// never matched against `resp_queue`. A push arriving when
/// `push_sender` is `None` (a non-pub/sub connection) or a push that
/// fails to parse is a hard connection error — failing loudly is
/// required because the alternative is dispatching the push as some
/// unrelated command's response.
///
/// # Errors
///
/// Returns [`io::Error::other`] wrapping a [`RedisError`] for any
/// non-`Parse` decode failure, for a pub/sub push on a connection
/// without a push channel, and for a malformed pub/sub push.
/// Partial-value parse errors are treated as benign and turned into
/// `Ok(())`.
pub(super) fn decode_responses(
    read_buf: &mut BytesMut,
    resp_queue: &mut VecDeque<PendingRequest>,
    pending_count: &Arc<AtomicUsize>,
    push_sender: Option<&spsc::Sender<super::pubsub::PubSubMessage>>,
) -> io::Result<()> {
    while !read_buf.is_empty() {
        let mut reader = RESPReader::new(read_buf.split());
        match reader.read_value() {
            Ok(value) => {
                read_buf.unsplit(reader.take_buf());
                // Pub/sub pushes must NEVER fall through to the pending-request
                // queue: dispatching a push as a command response silently
                // corrupts the positional FIFO matching for every later
                // command on this connection.
                if is_pubsub_push(&value) {
                    let Some(tx) = push_sender else {
                        log::error!(
                            "pub/sub push received on a non-pub/sub connection; \
                             use PubSubClient for subscriptions"
                        );
                        return Err(io::Error::other(RedisError::Protocol(
                            "pub/sub push received on a non-pub/sub connection \
                             (use PubSubClient for subscriptions)"
                                .into(),
                        )));
                    };
                    let Some(msg) = parse_pubsub_push(&value) else {
                        log::error!("malformed pub/sub push: {value:?}");
                        return Err(io::Error::other(RedisError::Protocol(format!(
                            "malformed pub/sub push: {value:?}"
                        ))));
                    };
                    if tx.send(msg).is_err() {
                        log::warn!(
                            "pub/sub subscriber receiver dropped; discarding push"
                        );
                    }
                    continue;
                }
                if let Some(pending) = resp_queue.pop_front() {
                    let _ = pending.sender.send(value);
                    release_pending(pending_count);
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

pub(super) fn error_dispatch(
    resp_queue: &mut VecDeque<PendingRequest>,
    pending_count: &Arc<AtomicUsize>,
    message: &str,
) {
    while let Some(pending) = resp_queue.pop_front() {
        let _ = pending.sender.send(RedisValue::Error(message.into()));
        release_pending(pending_count);
    }
}
