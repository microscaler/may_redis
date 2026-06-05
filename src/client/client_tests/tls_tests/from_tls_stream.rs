// Gap 4: from_tls_stream() integration tests.
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
    prepare_tls_tests, run_may, test_tls_config, tls_client, tls_port,
};
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;

#[test]
fn test_from_tls_stream_defaults() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("from_tls_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_from_tls_stream_with_ssrf_none() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("ssrf_none_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_from_tls_stream_with_ssrf_some() {
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
        .expect("TLS connection with SSRF should succeed");
        let val: Option<String> = client.execute(client.get("ssrf_some_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_from_tls_stream_connection_loop_starts() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        for i in 0..5 {
            let key = format!("loop_test_{i}");
            client.execute::<()>(client.set(&key, "val")).unwrap();
            let val: String = client.execute(client.get(&key)).unwrap();
            assert_eq!(val, "val");
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}
