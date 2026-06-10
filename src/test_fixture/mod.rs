#![cfg(feature = "test")]
//! Docker-managed Redis fixtures for integration tests (Epic 16).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_const_for_fn,
    clippy::missing_panics_doc,
    dead_code
)]

mod container;
mod runtime;

use std::sync::OnceLock;

pub use container::RedisContainer;
pub use container::RedisTestFixtureBuilder;

/// A managed Redis test fixture (one or more containers).
pub struct RedisTestFixture {
    containers: Vec<RedisContainer>,
}

impl RedisTestFixture {
    /// Create a builder for constructing a test fixture.
    #[must_use]
    pub fn builder() -> RedisTestFixtureBuilder {
        RedisTestFixtureBuilder::new()
    }

    /// Host port mapped for container at index `i`.
    #[must_use]
    pub fn host(&self, i: usize) -> u16 {
        self.containers[i].host_port
    }

    /// Number of containers in this fixture.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.containers.len()
    }

    /// Whether the fixture has no containers.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }
}

/// Error type for Docker build failures.
#[derive(Debug)]
pub enum DockerBuildError {
    /// Docker daemon is not running.
    DockerNotAvailable(String),
    /// Container creation failed.
    ContainerCreate(String),
    /// Container failed to start.
    ContainerStart(String),
    /// Container did not become ready in time.
    ContainerNotReady(String),
}

impl std::fmt::Display for DockerBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DockerNotAvailable(msg) => write!(f, "Docker not available: {msg}"),
            Self::ContainerCreate(msg) => write!(f, "Container creation failed: {msg}"),
            Self::ContainerStart(msg) => write!(f, "Container start failed: {msg}"),
            Self::ContainerNotReady(msg) => write!(f, "Container not ready: {msg}"),
        }
    }
}

impl std::error::Error for DockerBuildError {}

static FIXTURE: OnceLock<RedisTestFixture> = OnceLock::new();
static FIXTURE_INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Start Docker containers once per test process (safe on the test main thread).
///
/// Containers use fixed names and persist across test processes: they are
/// reused when already running instead of being recreated, so repeated test
/// runs never accumulate containers.
///
/// # Errors
///
/// Returns [`DockerBuildError`] if Docker is unavailable or fixture startup fails.
pub fn ensure_started() -> Result<(), DockerBuildError> {
    // Serialize initialization: without this, two test threads can both
    // build a fixture pointing at the same shared containers.
    let _guard = FIXTURE_INIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if FIXTURE.get().is_some() {
        return Ok(());
    }
    let fixture =
        runtime::run_on_docker_thread(|| RedisTestFixture::builder().build())?;
    if let Err(duplicate) = FIXTURE.set(fixture) {
        // Unreachable under the lock, but never drop a duplicate handle to
        // the shared containers.
        std::mem::forget(duplicate);
    }
    Ok(())
}

/// Shared fixture after [`ensure_started`].
pub fn shared_fixture() -> &'static RedisTestFixture {
    FIXTURE
        .get()
        .expect("call ensure_started() before using the Docker fixture")
}

/// Host port for plain Redis when the Docker fixture is running.
pub fn plain_redis_port() -> Option<u16> {
    FIXTURE.get().map(|f| f.host(0))
}

/// Host port for TLS Redis when the Docker fixture is running (index 1).
pub fn tls_redis_port() -> Option<u16> {
    FIXTURE.get().map(|fixture| {
        if fixture.len() > 1 {
            fixture.host(1)
        } else {
            fixture.host(0)
        }
    })
}

/// Returns true when Docker-based tests should be skipped.
#[must_use]
pub fn skip_docker_tests() -> bool {
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        eprintln!("SKIP: SKIP_DOCKER_TESTS is set");
        return true;
    }
    if let Err(err) = ensure_started() {
        eprintln!("SKIP: {err}");
        return true;
    }
    false
}
