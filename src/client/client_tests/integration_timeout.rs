#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{integration_redis_port, run_integration};
use crate::protocol::commands::ListsCommands;
use crate::PubSubClient;
use std::time::{Duration, Instant};

#[test]
fn test_integration_execute_with_timeout_fires() {
    run_integration(|| {
        // MUST use a dedicated connection: `BLPOP key 0` blocks the Redis
        // connection forever server-side. Issuing it on the shared client
        // would wedge every subsequent test (Redis serves a connection's
        // commands strictly in order). The wedged connection is dropped at
        // the end of this test.
        let port = integration_redis_port();
        let client =
            crate::RedisClient::connect("127.0.0.1", port).expect("timeout client");

        let start = Instant::now();
        let err = client
            .execute_with_timeout::<()>(
                client.blpop(&["may_redis_timeout_block"], 0),
                Duration::from_millis(100),
            )
            .unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "execute_with_timeout hung for {elapsed:?}"
        );
        assert!(
            format!("{err}").contains("timeout"),
            "expected timeout error, got {err}"
        );
    });
}

#[test]
fn test_integration_pubsub_recv_message_timeout_fires() {
    run_integration(|| {
        let port = integration_redis_port();
        let client = PubSubClient::connect("127.0.0.1", port).expect("pubsub connect");
        client
            .subscribe(&["may_redis_timeout_ch"])
            .expect("subscribe");

        let start = Instant::now();
        let err = client
            .recv_message_timeout(Duration::from_millis(100))
            .unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "recv_message_timeout hung for {elapsed:?}"
        );
        assert!(
            format!("{err}").contains("timed out"),
            "expected timeout error, got {err}"
        );

        client.unsubscribe_all().ok();
    });
}
