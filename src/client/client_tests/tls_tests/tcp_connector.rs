// Gap 13: TcpConnector integration tests.

use super::common::{run_may, test_tls_config};
use crate::RedisClient;
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};

const TLS_HOST: &str = "localhost";
const TLS_PORT: u16 = 6380;

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_default_timeout() {
    run_may(|| {
        let client = RedisClient::connect("127.0.0.1", 6379)
            .expect("Plain Redis connection should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_custom_timeout() {
    run_may(|| {
        let client = RedisClient::connect_with_timeout(
            "127.0.0.1", 6379, std::time::Duration::from_secs(10),
        ).expect("Plain Redis with 10s timeout should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_ssrf_check_early_block() {
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: true,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "10.0.0.1", TLS_PORT, &config, 2, ssrf,
        );
        assert!(result.is_err(), "Expected SSRF violation for private IP");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_timeout_conversion() {
    run_may(|| {
        let client = RedisClient::connect_with_timeout(
            "127.0.0.1", 6379, std::time::Duration::from_secs(1),
        ).expect("Plain Redis with 1s timeout should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_resolve_fails() {
    run_may(|| {
        let result = RedisClient::connect("nonexistent.invalid.host.localhost", 6379);
        assert!(result.is_err(), "Expected DNS resolution failure");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tcp_connect_to_closed_port() {
    run_may(|| {
        let result = RedisClient::connect("127.0.0.1", 65535);
        assert!(result.is_err(), "Expected connection error on closed port");
    });
}
