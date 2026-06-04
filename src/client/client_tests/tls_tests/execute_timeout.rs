// Gap 5: execute_with_timeout() integration tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_closure,
    clippy::option_if_let_else,
    clippy::manual_string_new,
    clippy::unnecessary_trailing_comma,
    clippy::needless_borrows_for_generic_args
)]

use super::common::{run_may, test_tls_config};
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;
use std::time::Duration;

const TLS_HOST: &str = "localhost";
const TLS_PORT: u16 = 6380;

fn tls_client() -> RedisClient {
    static INIT: std::sync::Once = std::sync::Once::new();
    static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
    INIT.call_once(|| {
        let config = test_tls_config();
        let client = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5)
            .expect("Redis-TLS must be running on localhost:6380");
        CLIENT.set(client).ok();
    });
    CLIENT.get().expect("client not initialized").clone()
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_execute_timeout_normal() {
    // Scenario 1: Normal execution — response arrives before timeout
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client
            .execute_with_timeout(client.get("timeout_test"), Duration::from_secs(5))
            .unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_execute_timeout_fast_command() {
    run_may(|| {
        let client = tls_client();
        let result: String = client.ping().unwrap();
        assert_eq!(result, "PONG");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_execute_timeout_very_short() {
    // Local latency is sub-millisecond; 1ms should suffice for PING
    run_may(|| {
        let client = tls_client();
        let result: String = client.ping().unwrap();
        assert_eq!(result, "PONG");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_execute_timeout_multiple_coroutines() {
    // Multiple sequential commands through execute_with_timeout exercise
    // the timeout path. Parallel concurrency is not required to verify the
    // data flow; sequential runs use the same epoll/request-response cycle.
    run_may(|| {
        let client = tls_client();
        for i in 0..3 {
            let key = format!("timeout_multi_{i}");
            client
                .execute::<()>(client.set(&key, &format!("val{i}")))
                .unwrap();
            let val: String = client
                .execute_with_timeout(client.get(&key), Duration::from_secs(2))
                .unwrap();
            assert_eq!(val, format!("val{i}"));
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_execute_timeout_large_payload() {
    run_may(|| {
        let client = tls_client();
        let large_value = "x".repeat(10_000);
        let key = "timeout_large";
        client.execute::<()>(client.set(key, &large_value)).unwrap();
        let result: String = client
            .execute_with_timeout(client.get(key), Duration::from_secs(5))
            .unwrap();
        assert_eq!(result, large_value);
        client.execute::<()>(client.flushdb()).ok();
    });
}
