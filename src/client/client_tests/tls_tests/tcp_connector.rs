// Gap 13: TcpConnector integration tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::common::{
    plain_port, prepare_tls_tests, run_may, test_tls_config, tls_port,
};
use crate::connection::tcp::SsrfConfig;
use crate::RedisClient;

#[test]
fn test_connect_default_timeout() {
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
fn test_connect_custom_timeout() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = RedisClient::connect_with_timeout(
            "127.0.0.1",
            plain_port(),
            std::time::Duration::from_secs(10),
        )
        .expect("Plain Redis with 10s timeout should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_connect_ssrf_check_early_block() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: true,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "10.0.0.1",
            tls_port(),
            &config,
            2,
            ssrf,
        );
        assert!(result.is_err(), "Expected SSRF violation for private IP");
    });
}

#[test]
fn test_connect_timeout_conversion() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = RedisClient::connect_with_timeout(
            "127.0.0.1",
            plain_port(),
            std::time::Duration::from_secs(1),
        )
        .expect("Plain Redis with 1s timeout should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_connect_resolve_fails() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        // `.invalid` is reserved (RFC 2606) and guaranteed to NXDOMAIN;
        // `.localhost` names resolve to loopback on systemd-resolved hosts.
        let result = RedisClient::connect("nonexistent.host.invalid", 6379);
        assert!(result.is_err(), "Expected DNS resolution failure");
    });
}

#[test]
fn test_connect_tcp_to_closed_port() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let result = RedisClient::connect("127.0.0.1", 65535);
        assert!(result.is_err(), "Expected connection error on closed port");
    });
}
