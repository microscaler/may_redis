// Gap 2: TlsConnector::handshake() integration tests.
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
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::tls::{
    config::{RustlsRootCerts, TlsVersion},
    TlsConfig,
};
use crate::RedisClient;

#[test]
fn test_handshake_success() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("handshake_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_handshake_timeout_no_tls() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", plain_port(), &config, 5);
        assert!(
            result.is_err(),
            "Expected TLS error connecting to non-TLS port"
        );
    });
}

#[test]
fn test_handshake_safety_valve() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", plain_port(), &config, 3);
        assert!(result.is_err(), "Expected TLS error");
    });
}

#[test]
fn test_handshake_invalid_ca() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let webpki_config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result =
            RedisClient::connect_tls("127.0.0.1", tls_port(), &webpki_config, 5);
        assert!(
            result.is_err(),
            "Expected cert verification failure with WebPkiRoots"
        );
    });
}

#[test]
fn test_handshake_wrong_server_name() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![tls_ca_cert_path()]),
            server_name: "wrong-server.example.com".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result = RedisClient::connect_tls("127.0.0.1", tls_port(), &config, 5);
        assert!(result.is_err(), "Expected cert verification failure");
    });
}

#[test]
fn test_handshake_empty_server_name_fallback() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![tls_ca_cert_path()]),
            server_name: String::new(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result = RedisClient::connect_tls("127.0.0.1", tls_port(), &config, 5);
        assert!(
            result.is_ok(),
            "Empty server_name should fallback to localhost"
        );
    });
}

#[test]
fn test_handshake_multiple_connections() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let config = test_tls_config();
        for i in 0..3 {
            let client = RedisClient::connect_tls("127.0.0.1", tls_port(), &config, 5);
            assert!(client.is_ok(), "TLS connection {i} should succeed",);
            let _ = client.unwrap().ping();
        }
    });
}
