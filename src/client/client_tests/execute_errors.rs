// execute()/execute_with_timeout() send-error propagation tests.
//
// Uses the in-process fake server from pipeline_errors plus a
// limit-constrained Connection to deterministically trigger send errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::pipeline_errors::spawn_fake_server;
use super::unit::run_may;
use crate::client::client::{InnerClient, RedisClient};
use crate::connection::Connection;
use crate::core::RedisError;
use crate::protocol::builder::{CommandBuilder, CommandPolicy};

use std::sync::Arc;
use std::time::Duration;

fn client_with_queue_depth(port: u16, max_queue_depth: usize) -> RedisClient {
    let connection = Connection::connect_with_limits(
        "127.0.0.1",
        port,
        Duration::from_secs(2),
        max_queue_depth,
        65536,
    )
    .expect("fake server connection failed");
    RedisClient {
        inner: Arc::new(InnerClient {
            connection,
            default_timeout: Duration::from_secs(2),
            command_policy: CommandPolicy::AllowAll,
        }),
    }
}

/// Negative: a send rejected by connection limits must surface as a
/// Connection error naming the real cause — not "response channel closed"
/// and not a misattributed "timeout".
#[test]
fn test_execute_surfaces_send_error() {
    run_may(|| {
        let port = spawn_fake_server(b"", 0);
        let client = client_with_queue_depth(port, 0);

        let err = client
            .execute::<String>(CommandBuilder::new("PING"))
            .unwrap_err();
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

/// Positive: a healthy connection still executes and decodes normally.
#[test]
fn test_execute_success_via_fake_server() {
    run_may(|| {
        let port = spawn_fake_server(b"+PONG\r\n", 1);
        let client = client_with_queue_depth(port, 1024);

        let pong: String = client
            .execute(CommandBuilder::new("PING"))
            .expect("execute should succeed");
        assert_eq!(pong, "PONG");
    });
}
