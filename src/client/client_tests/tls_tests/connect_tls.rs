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

use super::common::{
    plain_port, prepare_tls_tests, run_may, test_tls_config, tls_ca_cert_path,
    tls_client, tls_port,
};
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;

#[test]
fn test_connect_url_rediss() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let port = tls_port();
        let ca_path = tls_ca_cert_path();
        let ca = ca_path.to_string_lossy();
        let url = format!("rediss://127.0.0.1:{port}?ca_cert={ca}");
        let client = RedisClient::connect_url(&url).expect("rediss:// connect_url");
        assert_eq!(client.ping().unwrap(), "PONG");
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_connect_tls_success() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("connect_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_connect_tls_tcp_failure() {
    if !prepare_tls_tests() {
        return;
    }
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
fn test_connect_tls_tls_failure() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", plain_port(), &config, 2);
        assert!(result.is_err(), "Expected TLS error on plain Redis port");
    });
}

#[test]
fn test_connect_tls_with_ssrf_blocked() {
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
            5,
            ssrf,
        );
        assert!(result.is_err(), "Expected SSRF violation for 10.0.0.1");
    });
}

#[test]
fn test_connect_tls_with_ssrf_allowed_full_path() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let ssrf_allow = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "127.0.0.1",
            tls_port(),
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
fn test_connect_tls_with_ssrf_tcp_ok_tls_fail() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let ssrf_allow = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let result = RedisClient::connect_tls_with_ssrf(
            "127.0.0.1",
            plain_port(),
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
