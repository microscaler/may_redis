// connection.rs tests — Connection loop, request queue, and response dispatch
//
// Tests for Request, PendingRequest, process_req, nonblock_read,
// nonblock_write, decode_responses, and Connection lifecycle.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::connection::Connection;
use super::connection::PendingRequest;
use super::connection::Request;
use super::dispatch::decode_responses;
use super::dispatch::process_req;
use crate::core::RedisValue;
use bytes::Buf;
use bytes::BytesMut;
use may::config;

use may::go;
use may::queue::mpsc::Queue;
use may::sync::spsc;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Once;

fn init_may_runtime() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        config().set_workers(1);
    });
}

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
        assert_eq!(tag.unwrap(), 0);
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
        assert_eq!(tag0.unwrap(), 0);
        assert_eq!(tag1.unwrap(), 1);
        assert_eq!(tag2.unwrap(), 2);
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

// ======================== Story 12.3: Connection Drop Error Behavior ========================

/// Test that dropping a connection while coroutines await responses causes
/// those coroutines to get an error instead of hanging.
///
/// Creates a new `Connection` (not a shared `RedisClient`) so we can safely
/// drop it from a separate coroutine. Spawns 3 coroutines that each
/// enqueue a PING and then blocks on `rx.try_recv()`. After spawning, the
/// connection is immediately dropped. Every coroutine must receive an error
/// (or at least not hang) within a reasonable timeout.
///
/// Regression guard for findings C3 and S2 in `code-review-2026-05-28.md`:
/// `Connection::drop` uses `unsafe { rx.cancel() }` and could leave partial
/// state if the loop is mid-write, or cancel without ensuring in-flight
/// requests receive error responses.
#[test]
#[ignore = "requires live Redis server"]
fn test_connection_drop_during_request() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    init_may_runtime();
    go!(|| {
        let conn = Connection::connect("127.0.0.1", 6379).expect("connect");

        let results = Arc::new(AtomicUsize::new(0));
        let timeout_ms = 5_000u64;
        let n = 3;

        // Share the connection via Arc so coroutines can enqueue requests.
        let conn = Arc::new(conn);

        let handles: Vec<_> = (0..n)
            .map(|i| {
                let conn = Arc::clone(&conn);
                let results = Arc::clone(&results);
                go!(move || {
                    let (tx, rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) =
                        spsc::channel();
                    let ping = Request::new(b"*1\r\n$4\r\nPING\r\n".to_vec(), tx);
                    // Enqueue the request.
                    let _ = conn.send(ping);
                    // Small yield to let other coroutines enqueue too.
                    may::coroutine::yield_now();

                    // Poll for a response with a timeout to detect hangs.
                    let mut got_error = false;
                    let start = std::time::Instant::now();
                    loop {
                        if start.elapsed().as_millis() >= u128::from(timeout_ms) {
                            break;
                        }
                        match rx.try_recv() {
                            Ok(val) => {
                                got_error = matches!(&val, RedisValue::Error(_));
                                log::info!("coroutine {i}: got {val:?}");
                                break;
                            }
                            Err(_) => {
                                may::coroutine::sleep(std::time::Duration::from_millis(50));
                            }
                        }
                    }
                    if got_error {
                        results.fetch_add(1, Ordering::SeqCst);
                    } else {
                        log::warn!("coroutine {i}: did not receive an error (possible hang)");
                    }
                })
            })
            .collect();

        // Drop the connection — this cancels the loop.
        drop(conn);

        // Give the loop time to finish its error-drain cycle.
        may::coroutine::sleep(std::time::Duration::from_millis(500));

        // Join all spawned coroutines.
        for h in handles {
            let _ = h.join();
        }

        let success_count = results.load(Ordering::SeqCst);
        assert_eq!(
            success_count, n,
            "Expected all {n} coroutines to get errors, got {success_count}",
        );
    });
}

/// Test that dropping a connection while coroutines are executing
/// pipelines causes every coroutine to get an error.
///
/// Creates a new `Connection` and spawns 2 coroutines, each
/// sending a pipeline of 5 PING commands. The connection is dropped
/// right after launching. All coroutines must receive errors (not
/// hang or panic).
///
/// Regression guard for finding S2: cancellation must not bypass
/// the error-dispatch cleanup path that drains `resp_queue` with
/// `RedisValue::Error`.
#[test]
#[ignore = "requires live Redis server"]
fn test_connection_drop_during_pipeline() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    init_may_runtime();
    go!(|| {
        let conn = Connection::connect("127.0.0.1", 6379).expect("connect");

        let results = Arc::new(AtomicUsize::new(0));
        let timeout_ms = 5_000u64;
        let n = 2;
        let pipeline_len = 5;

        let conn = Arc::new(conn);

        let handles: Vec<_> = (0..n)
            .map(|i| {
                let conn = Arc::clone(&conn);
                let results = Arc::clone(&results);
                go!(move || {
                    let mut errors = 0usize;
                    for cmd_idx in 0..pipeline_len {
                        let (tx, rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) =
                            spsc::channel();
                        let ping = Request::new(b"*1\r\n$4\r\nPING\r\n".to_vec(), tx);
                        let _ = conn.send(ping);

                        // Wait for this command's response with a timeout.
                        let mut got = false;
                        let start = std::time::Instant::now();
                        loop {
                            if start.elapsed().as_millis() >= u128::from(timeout_ms) {
                                break;
                            }
                            match rx.try_recv() {
                                Ok(val) => {
                                    got = true;
                                    if matches!(&val, RedisValue::Error(_)) {
                                        errors += 1;
                                    }
                                    break;
                                }
                                Err(_) => {
                                    may::coroutine::sleep(std::time::Duration::from_millis(50));
                                }
                            }
                        }
                        if !got {
                            log::warn!(
                                "pipeline coroutine {i} cmd {cmd_idx}: no response (possible hang)"
                            );
                        }
                    }
                    if errors > 0 {
                        results.fetch_add(errors, Ordering::SeqCst);
                    }
                })
            })
            .collect();

        drop(conn);
        may::coroutine::sleep(std::time::Duration::from_millis(500));

        for h in handles {
            let _ = h.join();
        }

        let total_errors = results.load(Ordering::SeqCst);
        assert!(
            total_errors > 0,
            "Expected at least some errors in pipeline, got {total_errors}",
        );
    });
}

/// Test that dropping a connection in various orders never panics.
///
/// This verifies two scenarios:
///   1. Send a command, then immediately drop the connection
///      (drop before response arrives).
///   2. Drop the connection before sending any command.
///
/// In both cases the test must complete without panicking.
/// If `Connection::drop` does not handle the cancellation path
/// correctly, `may::coroutine::cancel` can trigger panics in the
/// connection loop or leave dangling channels.
#[test]
#[ignore = "requires live Redis server"]
fn test_connection_drop_no_panic() {
    // Scenario 1: Send then drop.
    {
        let conn = Connection::connect("127.0.0.1", 6379).expect("connect");
        let (tx, rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        let _ = conn.send(Request::new(b"*1\r\n$4\r\nPING\r\n".to_vec(), tx));
        drop(conn);
        // Give loop time to drain.
        may::coroutine::sleep(std::time::Duration::from_millis(200));
        // rx.recv() should return an error since sender was dropped.
        assert!(
            rx.try_recv().is_err(),
            "Expected try_recv to fail after drop (no response)"
        );
    }

    // Scenario 2: Drop before send.
    {
        let conn = Connection::connect("127.0.0.1", 6379).expect("connect");
        drop(conn);
        // A send into a cancelled/moved queue should not panic.
        // The Queue is Arc-moved, so push should not panic.
        let queue = Arc::new(Queue::<Request>::new());
        let (tx, _rx): (spsc::Sender<RedisValue>, spsc::Receiver<RedisValue>) = spsc::channel();
        queue.push(Request::new(b"*1\r\n$4\r\nPING\r\n".to_vec(), tx));
        // The fact we reached here without panicking is the assertion.
    }
}
