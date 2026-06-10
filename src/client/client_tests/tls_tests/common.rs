// Shared infrastructure for TLS integration tests.
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

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::tls::{
    config::{RustlsRootCerts, TlsVersion},
    TlsConfig,
};

/// One-time initialization of the may coroutine runtime.
///
/// Delegates to the crate-wide test init: the may config is global, so all
/// harnesses must agree on one workers/stack configuration.
fn init_may_runtime() {
    crate::client::client_tests::unit::init_may_runtime();
}

/// Run test logic inside the may scheduler.
///
/// TLS integration tests share one `tls_client()` and one Redis DB, and
/// call FLUSHDB for isolation — so they must not run concurrently. The
/// guard is held on the libtest thread (not a may worker), serializing
/// test bodies without affecting the coroutine scheduler.
pub(super) fn run_may<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    static ISOLATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

    init_may_runtime();
    let _guard = ISOLATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    crate::client::client_tests::unit::run_may(f)
}

/// Start Docker containers on the **test main thread** (before `run_may`).
pub(super) fn prepare_tls_tests() -> bool {
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        eprintln!("SKIP: SKIP_DOCKER_TESTS is set");
        return false;
    }
    match crate::test_fixture::ensure_started() {
        Ok(()) => true,
        Err(err) => {
            eprintln!("SKIP: {err}");
            false
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    )
}

/// Path to the test CA certificate (works on desktop NFS and ms02).
pub(super) fn tls_ca_cert_path() -> PathBuf {
    manifest_dir().join("tests/tls/ca.crt")
}

/// Build a [`TlsConfig`] using the repo test CA.
pub(super) fn test_tls_config() -> TlsConfig {
    TlsConfig {
        root_certificates: RustlsRootCerts::Pem(vec![tls_ca_cert_path()]),
        server_name: "localhost".to_string(),
        min_version: TlsVersion::Tls12,
        max_version: TlsVersion::Tls13,
        client_certs: None,
        verify_server: true,
    }
}

#[cfg(feature = "test")]
fn docker_fixture() -> &'static crate::test_fixture::RedisTestFixture {
    crate::test_fixture::shared_fixture()
}

/// Dynamic host port for the TLS Redis container (index 1 when plain+tls).
pub(super) fn tls_port() -> u16 {
    #[cfg(feature = "test")]
    {
        let fixture = docker_fixture();
        if fixture.len() > 1 {
            fixture.host(1)
        } else {
            fixture.host(0)
        }
    }
    #[cfg(not(feature = "test"))]
    {
        6380
    }
}

/// Dynamic host port for the plain Redis container (index 0).
pub(super) fn plain_port() -> u16 {
    #[cfg(feature = "test")]
    {
        docker_fixture().host(0)
    }
    #[cfg(not(feature = "test"))]
    {
        6379
    }
}

/// Shared TLS client connected to the Docker-managed Redis-TLS container.
///
/// Initialization is guarded by a `may` mutex, NOT `std::sync::Once`: this
/// function runs inside coroutines, and the handshake parks its coroutine.
/// A `Once` would block the worker *thread* in concurrent callers — with
/// `set_workers(1)` that deadlocks the whole scheduler.
pub(super) fn tls_client() -> crate::RedisClient {
    static CLIENT: OnceLock<crate::RedisClient> = OnceLock::new();
    static INIT_LOCK: OnceLock<may::sync::Mutex<()>> = OnceLock::new();

    let _guard = INIT_LOCK
        .get_or_init(|| may::sync::Mutex::new(()))
        .lock()
        .expect("tls_client init lock poisoned");
    if let Some(client) = CLIENT.get() {
        return client.clone();
    }
    let config = test_tls_config();
    let port = tls_port();
    let client = crate::RedisClient::connect_tls("127.0.0.1", port, &config, 5)
        .expect("Redis-TLS Docker fixture connection failed");
    CLIENT.set(client.clone()).ok();
    client
}
