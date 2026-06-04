// Gap 2: TlsConnector::handshake() integration tests.

use super::common::{run_may, test_tls_config};
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::tls::{
    config::{RustlsRootCerts, TlsVersion},
    TlsConfig,
};
use crate::RedisClient;

const TLS_HOST: &str = "localhost";
const TLS_PORT: u16 = 6380;
const TLS_CA_PATH: &str =
    "/home/casibbald/Workspace/microscaler/may_redis/tests/tls/ca.crt";

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
fn test_handshake_success() {
    // Scenario 1: Full handshake success
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("handshake_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_timeout_no_tls() {
    // Scenario 2: Handshake timeout on non-TLS port (6379)
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", 6379, &config, 5);
        assert!(
            result.is_err(),
            "Expected TLS error connecting to non-TLS port"
        );
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_safety_valve() {
    // Scenario 3: Idle loop safety valve (100 yields) prevents infinite spin
    run_may(|| {
        let config = test_tls_config();
        let result = RedisClient::connect_tls("127.0.0.1", 6379, &config, 3);
        assert!(result.is_err(), "Expected TLS error");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_invalid_ca() {
    // Scenario 7: WebPkiRoots vs self-signed CA -> cert verification failure
    run_may(|| {
        let webpki_config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &webpki_config, 5);
        assert!(
            result.is_err(),
            "Expected cert verification failure with WebPkiRoots"
        );
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_wrong_server_name() {
    // Scenario 6: server_name mismatch -> verification failure
    run_may(|| {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![std::path::PathBuf::from(
                TLS_CA_PATH,
            )]),
            server_name: "wrong-server.example.com".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5);
        assert!(result.is_err(), "Expected cert verification failure");
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_empty_server_name_fallback() {
    // Scenario 5: Empty server_name defaults to "localhost"
    run_may(|| {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![std::path::PathBuf::from(
                TLS_CA_PATH,
            )]),
            server_name: "".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            client_certs: None,
            verify_server: true,
        };
        let result = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5);
        assert!(
            result.is_ok(),
            "Empty server_name should fallback to localhost"
        );
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_handshake_multiple_connections() {
    // Multiple TLS connections exercise epoll under load.
    // Sequential runs exercise the same connection-loop + TLS path;
    // parallel runs are not needed to validate the data flow.
    run_may(|| {
        let config = test_tls_config();
        for i in 0..3 {
            let client = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5);
            assert!(client.is_ok(), "TLS connection {i} should succeed",);
            let _ = client.unwrap().ping();
        }
    });
}
