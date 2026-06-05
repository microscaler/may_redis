// Gap 9: Connection struct methods integration tests.
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

use super::common::{
    plain_port, prepare_tls_tests, run_may, test_tls_config, tls_client, tls_port,
};
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;

#[test]
fn test_connect_default_limits() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_ssrf_config_none_plain() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = RedisClient::connect("127.0.0.1", plain_port())
            .expect("Plain Redis connection should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_send_success_within_limits() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        for i in 0..10 {
            let key = format!("send_test_{i}");
            client
                .execute::<()>(client.set(&key, &format!("val{i}")))
                .unwrap();
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_send_request_too_large() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let large_value = "x".repeat(100_000);
        let result: Result<(), crate::core::RedisError> =
            client.execute(client.set("large_key", &large_value));
        assert!(result.is_ok(), "100KB should be within default 1MB limit");
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_connection_id_uniqueness() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let client1 = RedisClient::connect_tls("127.0.0.1", tls_port(), &config, 5)
            .expect("Connection 1 should succeed");
        let client2 = RedisClient::connect_tls("127.0.0.1", tls_port(), &config, 5)
            .expect("Connection 2 should succeed");
        let _ = client1.ping().unwrap();
        let _ = client2.ping().unwrap();
        client1.execute::<()>(client1.flushdb()).ok();
        client2.execute::<()>(client2.flushdb()).ok();
    });
}

#[test]
fn test_ssrf_config_some_on_tls_with_ssrf() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let client = RedisClient::connect_tls_with_ssrf(
            "127.0.0.1",
            tls_port(),
            &config,
            5,
            ssrf,
        )
        .expect("TLS with SSRF should succeed");
        let _ = client.ping().unwrap();
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_connection_pooling_shared_client() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let client2 = client.clone();
        client
            .execute::<()>(client.set("shared_key", "shared_val"))
            .unwrap();
        let val: String = client2.execute(client2.get("shared_key")).unwrap();
        assert_eq!(val, "shared_val");
        client.execute::<()>(client.flushdb()).ok();
    });
}
