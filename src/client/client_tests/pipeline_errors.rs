// Pipeline error-propagation tests (no real Redis server needed).
//
// Uses an in-process std::net::TcpListener as a fake Redis server so the
// connection loop has a live socket, plus Connection::connect_with_limits
// to deterministically trigger send-side limit errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::run_may;
use crate::client::pipeline::Pipeline;
use crate::connection::Connection;
use crate::core::{RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

/// Encoded `PING` is exactly 14 bytes: `*1\r\n$4\r\nPING\r\n`.
const PING_WIRE_LEN: usize = 14;

/// Spawn a fake Redis server on an ephemeral port.
///
/// Accepts one connection, reads `expected_cmds` PING commands, writes
/// `responses`, then holds the socket open until the client disconnects.
fn spawn_fake_server(responses: &'static [u8], expected_cmds: usize) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let mut got = 0;
            while got < expected_cmds * PING_WIRE_LEN {
                match sock.read(&mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => got += n,
                }
            }
            let _ = sock.write_all(responses);
            let _ = sock.flush();
            // Keep the connection open until the client goes away.
            let _ = sock.read(&mut buf);
        }
    });
    port
}

fn connect(port: u16, max_queue_depth: usize) -> Connection {
    Connection::connect_with_limits(
        "127.0.0.1",
        port,
        Duration::from_secs(2),
        max_queue_depth,
        65536,
    )
    .expect("fake server connection failed")
}

/// Negative: a send rejected by connection limits must surface as a
/// Connection error naming the cause — not the misleading
/// Parse("response channel closed").
#[test]
fn test_pipeline_execute_raw_surfaces_send_error() {
    run_may(|| {
        let port = spawn_fake_server(b"", 0);
        // Queue depth 0: every send fails with QueueFull immediately.
        let conn = connect(port, 0);
        let mut pipeline = Pipeline::new(&conn);
        pipeline.add(CommandBuilder::new("PING"));

        let err = pipeline.execute_raw().unwrap_err();
        match err {
            RedisError::Connection(msg) => {
                assert!(
                    msg.contains("full"),
                    "error must name the real cause (queue full), got: {msg}"
                );
            }
            other => panic!("expected RedisError::Connection, got {other:?}"),
        }
    });
}

/// Negative: execute_raw_results must return per-command errors for failed
/// sends instead of spinning forever waiting for responses that can never
/// arrive.
#[test]
fn test_pipeline_execute_raw_results_send_error_no_hang() {
    run_may(|| {
        let port = spawn_fake_server(b"", 0);
        let conn = connect(port, 0);
        let mut pipeline = Pipeline::new(&conn);
        pipeline.add(CommandBuilder::new("PING"));
        pipeline.add(CommandBuilder::new("PING"));

        let results = pipeline.execute_raw_results();
        assert_eq!(results.len(), 2);
        for result in results {
            let err = result.expect_err("failed send must produce Err");
            assert!(
                matches!(err, RedisError::Connection(ref msg) if msg.contains("full")),
                "expected Connection(queue full), got {err:?}"
            );
        }
    });
}

/// Positive: a healthy pipeline still delivers all responses in order.
#[test]
fn test_pipeline_execute_raw_success() {
    run_may(|| {
        let port = spawn_fake_server(b"+PONG\r\n+PONG\r\n", 2);
        let conn = connect(port, 1024);
        let mut pipeline = Pipeline::new(&conn);
        pipeline.add(CommandBuilder::new("PING"));
        pipeline.add(CommandBuilder::new("PING"));

        let values = pipeline.execute_raw().expect("pipeline should succeed");
        assert_eq!(values.len(), 2);
        for value in values {
            assert!(matches!(value, RedisValue::SimpleString(ref s) if s == "PONG"));
        }
    });
}

/// Positive: execute_raw_results returns Ok entries on the happy path.
#[test]
fn test_pipeline_execute_raw_results_success() {
    run_may(|| {
        let port = spawn_fake_server(b"+PONG\r\n+PONG\r\n", 2);
        let conn = connect(port, 1024);
        let mut pipeline = Pipeline::new(&conn);
        pipeline.add(CommandBuilder::new("PING"));
        pipeline.add(CommandBuilder::new("PING"));

        let results = pipeline.execute_raw_results();
        assert_eq!(results.len(), 2);
        for result in results {
            let value = result.expect("healthy pipeline entry must be Ok");
            assert!(matches!(value, RedisValue::SimpleString(ref s) if s == "PONG"));
        }
    });
}
