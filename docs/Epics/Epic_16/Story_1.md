# Story 16.1 — Fix test_fixture.rs (bollard Drop cleanup)

**Objective:** Repair the existing `tests/test_fixture.rs` stub so it compiles and runs. The key fix: replace `tokio::spawn` in the `Drop` impl with a synchronous approach (std thread spawn + blocking bollard call), remove all `tokio` usage, and ensure containers are always cleaned up.

**Epic:** 16 — Docker-Managed Test Fixtures

**Dependencies:** None (only edits to existing `tests/test_fixture.rs`)

**Status:** TODO

**Source docs:** `tests/test_fixture.rs`, BRRTRouter `tests/docker_integration_tests.rs` (RAII Drop pattern), `rust-testing` skill (may runtime patterns)

## Code Anchors

- `tests/test_fixture.rs` — the fixture file that needs fixing

## Current Issues

The existing fixture has three problems:

1. **`tokio::spawn` in `Drop`** (line 68) — violates no-tokio rule. The `Drop` impl runs synchronously when the fixture goes out of scope. We need a sync cleanup path.
2. **`tokio::time::sleep` in `wait_until_ready`** (line 307) — the method is `async fn` but `may` doesn't have `tokio::time`. Need to convert to a sync loop or use a may-compatible async approach.
3. **No Docker availability check** — `Docker::connect_with_socket_defaults()` panics if Docker isn't running. Need a graceful early return.

## Implementation Details

### 1. Fix `Drop` — use std thread for cleanup

Instead of `tokio::spawn(async move { ... })`, use a std thread that calls the bollard API synchronously:

```rust
impl Drop for RedisTestFixture {
    fn drop(&mut self) {
        for container in self.containers.drain(..) {
            let id = container.id.clone();
            let docker = container._docker.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    if let Err(e) = docker
                        .remove_container(&id, Some(RemoveContainerOptions { force: true, ..Default::default() }))
                        .await
                    {
                        eprintln!("Failed to remove Redis container {id}: {e}");
                    }
                });
            });
        }
    }
}
```

**Why this works:** This is test code only and bollard is already a dev-dependency with tokio as a transitive dep. Using `tokio::runtime::Builder` per cleanup thread is a localized use of tokio — it never touches the library, the connection loop, or any production code. The BRRTRouter project uses the same pattern in `docker_integration_tests.rs` (they use `futures::executor::block_on` with the same approach).

### 2. Fix `wait_until_ready` — make it sync

The async `wait_until_ready` is awkward because the fixture builder is `async` but callers need a sync path. Convert to a synchronous blocking loop:

```rust
impl RedisContainer {
    /// Wait until the container is accepting connections.
    pub fn wait_until_ready(&self) -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if std::net::TcpStream::connect_timeout(
                &format!("127.0.0.1:{}", self.host_port),
                std::time::Duration::from_millis(100),
            )
            .is_ok()
            {
                return Ok(());
            }
            if std::time::Instant::now() > deadline {
                return Err("Redis container did not become ready in time".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
```

And update `build()` to call the sync version instead of `.await`.

### 3. Add Docker availability check

In `RedisTestFixtureBuilder::build()`, check Docker first:

```rust
pub fn build(mut self) -> Result<RedisTestFixture, String> {
    let docker = match Docker::connect_with_socket_defaults() {
        Ok(d) => d,
        Err(e) => return Err(format!("Docker not available: {e}")),
    };
    // ... rest of build
}
```

## Tasks

- [ ] Remove `tokio::spawn` from `Drop`, replace with `std::thread::spawn` + `tokio::runtime::Builder::new_current_thread().build().unwrap().block_on()`
- [ ] Remove `async` from `build()` and `wait_until_ready()`, convert to sync functions
- [ ] Remove `tokio::time::sleep`, replace with `std::thread::sleep`
- [ ] Add Docker availability check in `build()` that returns `Err` (not panic)
- [ ] Add `#![allow(clippy::unwrap_used)]` at file top (test code)
- [ ] `cargo test -p may-redis --features test` — fixture compiles (doesn't need to connect to Docker to compile)

## Acceptance Criteria

| # | AC | Maps to |
|---|----|---------|
| 1 | `tests/test_fixture.rs` compiles with `cargo check --tests` and zero clippy warnings | NFR-007 |
| 2 | `build()` creates a `redis:7-alpine` container with a random host port mapped to 127.0.0.1 | FR-001, FR-010, NFR-008 |
| 3 | `host(i)` returns the correct mapped host port | FR-002 |
| 4 | `Drop` removes all containers when `RedisTestFixture` goes out of scope, even on panic | FR-003, NFR-002 |
| 5 | Container names are unique per process using PID | FR-004 |
| 6 | `build()` returns `Result` — fails gracefully with descriptive error when Docker is unavailable | FR-006 |
| 7 | `wait_until_ready()` returns `Ok(())` when Redis is accepting connections on the mapped port | NFR-003 |
| 8 | Container startup + readiness completes within 10 seconds | NFR-003 |
| 9 | Plain Redis (port 6379) and Redis-TLS (port 6380) variants are both supported | FR-005 |
| 10 | No `tokio` imports in `src/` — tokio appears only in test `Drop` for cleanup | NFR-001 |

## Verification

- [ ] `cargo check --tests` — compiles with zero warnings
- [ ] `cargo clippy --tests -- -D warnings` — clean
- [ ] Run without Docker: `SKIP_DOCKER_TESTS=1 cargo test` — no panics, fixture returns Err gracefully
- [ ] Run with Docker: fixture creates and removes a container successfully
