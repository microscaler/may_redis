//! Standalone test to verify the bollard fixture works end-to-end.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod test_fixture;

#[test]
fn test_fixture_end_to_end() {
    if !test_fixture::is_docker_available() {
        eprintln!("SKIP: Docker not available");
        return;
    }

    let fixture = test_fixture::RedisTestFixture::builder()
        .with_plain_redis(true)
        .with_tls_redis(false)
        .build()
        .expect("failed to build fixture");

    assert_eq!(fixture.len(), 1);
    let port = fixture.host(0);
    eprintln!("Plain Redis container ready on port {port}");

    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("valid SocketAddr");
    let result =
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5));
    assert!(result.is_ok(), "Should connect on port {port}");

    drop(fixture);
    eprintln!("Fixture dropped — containers cleaned up");
}

#[test]
fn test_fixture_both_containers() {
    if !test_fixture::is_docker_available() {
        eprintln!("SKIP: Docker not available");
        return;
    }

    let fixture = test_fixture::RedisTestFixture::builder()
        .with_plain_redis(true)
        .with_tls_redis(true)
        .build()
        .expect("failed to build fixture with TLS");

    assert_eq!(fixture.len(), 2);
    let plain_port = fixture.host(0);
    let tls_port = fixture.host(1);
    eprintln!("Plain Redis on port {plain_port}, TLS Redis on port {tls_port}");

    for (name, port) in &[(plain_port, "plain"), (tls_port, "tls")] {
        let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
            .parse()
            .expect("valid SocketAddr");
        let result = std::net::TcpStream::connect_timeout(
            &addr,
            std::time::Duration::from_secs(5),
        );
        assert!(result.is_ok(), "Should connect to {name} on port {port}");
    }

    drop(fixture);
    eprintln!("Both containers dropped — cleaned up");
}
