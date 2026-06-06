//! Standalone test to verify the bollard fixture works end-to-end.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "test")]

#[cfg(feature = "tls")]
use may::config;
#[cfg(feature = "tls")]
use may::go;
#[cfg(feature = "tls")]
use may_redis::tls::config::{RustlsRootCerts, TlsVersion};
#[cfg(feature = "tls")]
use may_redis::tls::TlsConfig;
#[cfg(feature = "tls")]
use may_redis::RedisClient;
#[cfg(feature = "tls")]
use std::sync::{Arc, Mutex, Once};

mod test_fixture;

#[cfg(feature = "tls")]
static TLS_INIT: Once = Once::new();

#[test]
fn test_fixture_end_to_end() {
    if test_fixture::skip_docker_tests() {
        return;
    }

    let fixture = test_fixture::shared_fixture();
    assert!(!fixture.is_empty());
    let port = fixture.host(0);
    eprintln!("Plain Redis container ready on port {port}");

    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("valid SocketAddr");
    let result =
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5));
    assert!(result.is_ok(), "Should connect on port {port}");
}

#[test]
fn test_fixture_both_containers() {
    if test_fixture::skip_docker_tests() {
        return;
    }

    let fixture = test_fixture::shared_fixture();
    assert_eq!(fixture.len(), 2);
    let plain_port = fixture.host(0);
    let tls_port = fixture.host(1);
    eprintln!("Plain Redis on port {plain_port}, TLS Redis on port {tls_port}");

    for (label, port) in [("plain", plain_port), ("tls", tls_port)] {
        let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
            .parse()
            .expect("valid SocketAddr");
        let result = std::net::TcpStream::connect_timeout(
            &addr,
            std::time::Duration::from_secs(5),
        );
        assert!(result.is_ok(), "Should connect to {label} on port {port}");
    }
}

/// Verify the TLS container accepts a rustls handshake and responds to PING.
#[cfg(feature = "tls")]
#[test]
fn test_fixture_tls_handshake_ping() {
    if test_fixture::skip_docker_tests() {
        return;
    }

    TLS_INIT.call_once(|| {
        config().set_stack_size(64 * 1024);
        config().set_workers(1);
    });

    let tls_port = test_fixture::tls_redis_port().expect("TLS fixture port");
    let ca_path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("manifest"),
    )
    .join("tests/tls/ca.crt");

    let tls_config = TlsConfig {
        root_certificates: RustlsRootCerts::Pem(vec![ca_path]),
        server_name: "localhost".to_string(),
        min_version: TlsVersion::Tls12,
        max_version: TlsVersion::Tls13,
        client_certs: None,
        verify_server: true,
    };

    let result = Arc::new(Mutex::new(None::<String>));
    let result2 = Arc::clone(&result);

    let handle = go!(move || {
        let client = RedisClient::connect_tls("127.0.0.1", tls_port, &tls_config, 5)
            .expect("TLS connect");
        *result2.lock().expect("lock") = client.ping().ok();
    });

    handle.join().expect("join");
    assert_eq!(
        result.lock().expect("lock").as_deref(),
        Some("PONG"),
        "TLS Redis fixture should respond to PING"
    );
}
