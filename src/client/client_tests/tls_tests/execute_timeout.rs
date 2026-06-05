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

use super::common::{prepare_tls_tests, run_may, tls_client};
use crate::protocol::commands::{AdminCommands, StringsCommands};
use std::time::Duration;

#[test]
fn test_execute_timeout_normal() {
    if !prepare_tls_tests() {
        return;
    }
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
fn test_execute_timeout_fast_command() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let result: String = client.ping().unwrap();
        assert_eq!(result, "PONG");
    });
}

#[test]
fn test_execute_timeout_very_short() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let result: String = client.ping().unwrap();
        assert_eq!(result, "PONG");
    });
}

#[test]
fn test_execute_timeout_multiple_coroutines() {
    if !prepare_tls_tests() {
        return;
    }
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
fn test_execute_timeout_large_payload() {
    if !prepare_tls_tests() {
        return;
    }
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
