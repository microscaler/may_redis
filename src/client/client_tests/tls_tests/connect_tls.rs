// Gap 3: connect_tls() / connect_tls_with_ssrf() integration tests.
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
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;

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
fn test_connect_tls_success() {
    // Scenario 1: Happy path
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("connect_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_tls_tcp_failure() {
    // Scenario 2: TCP connect fails (no server)
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", 65535, &config, 2);
        assert!(
            result.is_err(),
            "Expected connection error on non-listening port"
        );
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_tls_tls_failure() {
    // Scenario 3: TCP connects, TLS handshake fails (plain Redis on 6379)
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", 6379, &config, 2);
        assert!(result.is_err(), "Expected TLS error on plain Redis port");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_tls_with_ssrf_blocked() {
    // Scenario 4: SSRF blocks 10.0.0.1 before TCP
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: true,
        };
        let result =
            RedisClient::connect_tls_with_ssrf("10.0.0.1", TLS_PORT, &config, 5, ssrf);
        assert!(result.is_err(), "Expected SSRF violation for 10.0.0.1");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_tls_with_ssrf_allowed_full_path() {
    // Scenario 5: SSRF allows, TCP connects, TLS handshakes
    run_may(|| {
        let config = test_tls_config();
        let ssrf_allow = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "127.0.0.1",
            TLS_PORT,
            &config,
            5,
            ssrf_allow,
        );
        assert!(
            result.is_ok(),
            "TLS connection with SSRF allowed should succeed"
        );
        if let Ok(client) = result {
            let _ = client.ping();
        }
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_tls_with_ssrf_tcp_ok_tls_fail() {
    // Scenario 6: SSRF allows, TCP connects, TLS handshake fails
    run_may(|| {
        let config = test_tls_config();
        let ssrf_allow = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "127.0.0.1",
            6379,
            &config,
            2,
            ssrf_allow,
        );
        assert!(
            result.is_err(),
            "Expected TLS error on plain Redis with SSRF allowed"
        );
    });
}
