# Story 16.3 — Docker availability check + skip logic

**Objective:** Add graceful skip logic so integration tests don't fail when Docker is unavailable. Instead of panicking, tests check for Docker, skip themselves with a clear message, and allow the test suite to run partially.

**Epic:** 16 — Docker-Managed Test Fixtures

**Dependencies:** Story 16.1 (test_fixture.rs fixed), Story 16.2 (fixture wired into tests)

**Status:** TODO

**Source docs:** `tests/test_fixture.rs`, `src/client/client_tests/unit.rs`, BRRTRouter `tests/docker_integration_tests.rs` (E2E_DOCKER env var check)

## Problem

When Docker is not installed or not running:
- `Docker::connect_with_socket_defaults()` returns `Err`
- The fixture's `build()` panics or the caller panics
- The entire test suite fails (or worse, hangs)
- CI/CD pipelines can't run `cargo test` without a Docker dependency

## Solution

Implement a two-layer skip mechanism:

### Layer 1: Fixture-level Docker check

In `RedisTestFixtureBuilder::build()`:

```rust
pub fn build(mut self) -> Result<RedisTestFixture, String> {
    // Check Docker availability first
    if let Err(e) = Docker::connect_with_socket_defaults() {
        return Err(format!("Docker not available: {e}. Set SKIP_DOCKER_TESTS=1 to skip."));
    }
    
    let docker = Docker::connect_with_socket_defaults().unwrap();
    // ... rest of build
}
```

### Layer 2: Test-level skip

In each test file, wrap test logic with a skip check:

```rust
#[test]
#[ignore = "requires Docker + Redis container"]
fn test_strings_get() {
    // Skip if Docker not available
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        println!("SKIPPED: Docker not available (SKIP_DOCKER_TESTS=1)");
        return;
    }
    
    let fixture = RedisTestFixture::builder()
        .with_plain_redis(true)
        .with_tls_redis(false)
        .build();
    
    let client = RedisClient::connect("127.0.0.1", fixture.host(0));
    run_may(|| {
        // test logic
    });
}
```

### Alternative: Feature flag approach

Use a Cargo feature flag `docker-test` that gates integration tests:

```toml
[features]
default = []
docker-test = []
```

Then in test files:
```rust
#[cfg(feature = "docker-test")]
mod integration_tests {
    // all integration tests here
}
```

**Decision: Skip via env var, NOT feature flag.** Feature flags require `--features docker-test` for CI, which adds complexity. Env var skip (`SKIP_DOCKER_TESTS=1`) is simpler — tests compile, run, and skip themselves with a clear message.

## Implementation Details

### 1. Add `is_docker_available()` helper

In `tests/test_fixture.rs`:

```rust
/// Check if Docker is available and running.
pub fn is_docker_available() -> bool {
    Docker::connect_with_socket_defaults().is_ok()
}
```

### 2. Add `DockerBuildError` type

Create a proper error type for Docker build failures:

```rust
#[derive(Debug)]
pub enum DockerBuildError {
    /// Docker daemon is not running
    DockerNotAvailable(String),
    /// Container creation failed
    ContainerCreate(String),
    /// Container failed to start
    ContainerStart(String),
    /// Container did not become ready in time
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
```

### 3. Update all integration test files

Each integration test file needs a module-level skip function:

```rust
// At the top of each integration test file
mod skip_if_no_docker {
    use crate::tests::test_fixture;
    
    pub fn check() {
        if !test_fixture::is_docker_available() {
            println!("SKIPPED: Docker not available. Set SKIP_DOCKER_TESTS=1 to silence.");
            std::process::exit(0);  // or return from each test
        }
    }
}
```

Actually — better approach: use `#[should_panic]` for the skip check, or use a macro. But the simplest and most Rust-idiomatic is to check in each test individually (not at module level) because:

1. `cargo test` runs each test in its own process, so checking once per test is fine
2. It's explicit — each test knows it needs Docker
3. No shared state or module-level side effects

So the pattern in each test is:

```rust
#[test]
#[ignore = "requires Docker + Redis container"]
fn test_strings_get() {
    if !test_fixture::is_docker_available() {
        println!("SKIPPED: Docker not available");
        return;
    }
    // ... rest of test
}
```

### 4. Performance consideration

Checking `Docker::connect_with_socket_defaults()` 100+ times per test run is unnecessary. Add a cached check:

```rust
use std::sync::OnceLock;

static DOCKER_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn is_docker_available() -> bool {
    *DOCKER_AVAILABLE.get_or_init(|| Docker::connect_with_socket_defaults().is_ok())
}
```

This checks Docker once per test process, not per test.

## Tasks

- [ ] Add `DockerBuildError` type to `test_fixture.rs`
- [ ] Add `is_docker_available()` with `OnceLock` caching to `test_fixture.rs`
- [ ] Update `build()` to return `Result<RedisTestFixture, DockerBuildError>`
- [ ] Add skip check to `integration_admin_basic.rs` (2-3 tests)
- [ ] Add skip check to `integration_admin_advanced.rs` (3-4 tests)
- [ ] Add skip check to `integration_strings_basic.rs` (~15 tests)
- [ ] Add skip check to `integration_strings_advanced.rs` (~15 tests)
- [ ] Add skip check to `integration_hashes_basic.rs` (~10 tests)
- [ ] Add skip check to `integration_hashes_advanced.rs` (~8 tests)
- [ ] Add skip check to `integration_lists_basic.rs` (~12 tests)
- [ ] Add skip check to `integration_sets_basic.rs` (~12 tests)
- [ ] Add skip check to `integration_sorted_sets.rs` (~10 tests)
- [ ] Add skip check to `integration_transactions.rs` (~8 tests)

## Acceptance Criteria

| # | AC | Maps to |
|---|----|---------|
| 1 | `is_docker_available()` returns `true` when Docker is running, `false` when not | FR-007 |
| 2 | `is_docker_available()` is cached via `OnceLock` — one Docker connection per test process | NFR-005 |
| 3 | `SKIP_DOCKER_TESTS=1` env var causes all integration tests to skip with clear `SKIPPED:` message | FR-009 |
| 4 | `build()` returns `Result<RedisTestFixture, DockerBuildError>` with descriptive error | FR-006 |
| 5 | Test skip completes in under 1 second — no blocking on unavailable Docker | NFR-004 |
| 6 | All 10 integration test files include skip check | FR-009 |
| 7 | Without `--include-ignored`, integration tests are ignored (not failing) | NFR-007 |

## Verification

- [ ] With Docker: `cargo test --lib -- --include-ignored test_integration_` — all tests pass
- [ ] Without Docker: `SKIP_DOCKER_TESTS=1 cargo test --lib` — all tests skip (not fail)
- [ ] `cargo test --lib` (no --include-ignored) — unit tests pass, integration tests ignored
- [ ] `cargo clippy --tests -- -D warnings` — clean
