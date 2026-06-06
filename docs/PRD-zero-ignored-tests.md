# PRD: Eliminate All Ignored Tests (Zero Ignored Tests Goal)

> **Epic 17** — Zero ignored tests
> **Target:** 0 ignored tests across all suites (currently 13)
> **Prerequisite:** Docker fixture infrastructure (feature `test`) — v0.1.0 stable
> **Scope:** 13 ignored tests in 4 locations, 1 doctest

---

## Problem Statement

The may-redis test suite has **13 ignored tests** that are not running in CI:

| Suite | Ignored | Notes |
|-------|---------|-------|
| Unit tests (main) | 12 | All in `connection/` — connection lifecycle, resource limits, error handling |
| Doctests | 1 | `cluster/cluster_client.rs` — `ignore` on broken example code |

The ignored tests are **regression guards** for critical path scenarios:
- Connection lifecycle (connect, drop, drop-during-pipeline, drop-during-request)
- Resource limits (queue-full, request-too-large)
- Error handling (connect refused returns proper error, not hang/panic)

These are exactly the kinds of bugs that caused production hangs (see connection-loop-pitfalls). They should be running in CI on every commit.

---

## Current Test Infrastructure (Already Working)

v0.1.0 has a fully functional Docker fixture system:

| Component | Location | Purpose |
|-----------|----------|---------|
| `RedisTestFixture` | `src/test_fixture/mod.rs` | RAII Docker container management |
| `skip_docker_tests()` | Same | Graceful skip when Docker unavailable |
| `shared_fixture()` | Same | Lazy-initialized single fixture per test process |
| `ensure_started()` | Same | One-time Docker container boot |
| `run_integration()` | `src/client/client_tests/unit.rs` | `may::run` / `may::go!` test harness |
| `shared_client()` | Same | Shared `RedisClient` via `OnceLock` |
| `.nextest.toml` | Repo root | Test ordering, coroutine stability |

**Integration tests using this pattern:** 100+ tests under `src/client/client_tests/` — all passing, all using `run_integration` + `shared_client` + `FLUSHDB` isolation.

**The gap:** The ignored tests in `src/connection/` use `Connection::connect()` directly instead of `RedisClient`, and have never been wired into the fixture system.

---

## Solution Architecture

Convert each ignored test to use the existing fixture pattern:

```rust
// Pattern for connection-layer tests needing a real Redis server:
#[test]
fn test_something() {
    if test_fixture::skip_docker_tests() {
        return;
    }
    let port = test_fixture::plain_redis_port().expect("fixture port");
    run_integration(|| {
        let Ok(conn) = Connection::connect("127.0.0.1", port) else {
            return;
        };
        // ... test logic ...
    });
}
```

For tests that need a **refused connection** (not just "no Redis"):
- Use a random ephemeral port: `let port = get_random_unused_port();`
- The test connects to that port, expects `ConnectionError::Connect`, and verifies it's a proper error (not hang/panic)

---

## Implementation Plan

### Story 17.1: Wire connection lifecycle tests to Docker fixture

**Files changed:** `src/connection/connection_tests.rs`

| # | Test | What it validates | Conversion |
|---|------|-------------------|------------|
| 1 | `test_connection_connect` | `Connection::connect()` returns valid connection with non-zero ID | Use `plain_redis_port()` |
| 2 | `test_connection_send_tags` | Monotonically increasing request tags (0, 1, 2) | Use `plain_redis_port()` |
| 3 | `test_connection_drop` | Drop cleans up without hang or leak | Use `plain_redis_port()` |
| 4 | `test_connection_id` | `Connection::id()` returns non-zero | Use `plain_redis_port()` |

**Pattern:**

```rust
#[test]
fn test_connection_connect() {
    if test_fixture::skip_docker_tests() {
        return;
    }
    let port = test_fixture::plain_redis_port().expect("fixture port");
    run_integration(|| {
        let conn = Connection::connect("127.0.0.1", port);
        if let Ok(c) = conn {
            assert!(c.id() > 0);
            let tag = c.send(Request::new(vec![0], spsc::channel().0));
            assert_eq!(tag.unwrap(), 0);
        }
    });
}
```

**Verification:** Run `cargo test --workspace --features test connection::connection_tests::test_connection_connect` and confirm it runs (not ignored).

---

### Story 17.2: Wire connection drop/cancellation regression tests to Docker fixture

**Files changed:** `src/connection/connection_tests.rs`

These are the most critical tests — they guard against the production hangs documented in `llmwiki/topics/connection-loop-pitfalls.md`.

| # | Test | What it validates | Conversion |
|---|------|-------------------|------------|
| 5 | `test_connection_drop_during_pipeline` | Cancelling loop during pipeline drains resp_queue with RedisValue::Error | Use `plain_redis_port()` |
| 6 | `test_connection_drop_during_request` | Cancelling loop during single request drains resp_queue | Use `plain_redis_port()` |
| 7 | `test_connection_drop_no_panic` | Send-then-drop completes without panic | Use `plain_redis_port()` |

**Pattern:** Same as Story 17.1 — swap the port, keep the test logic identical. These tests are already correct; they just need a live server to exercise.

---

### Story 17.3: Wire connection error handling tests to Docker fixture

**Files changed:** `src/connection/connection_tests.rs`

| # | Test | What it validates | Conversion |
|---|------|-------------------|------------|
| 8 | `test_connection_send_tags` | (moved from 17.1) Monotonically increasing tags | Covered in 17.1 |

**Note:** `test_connection_send_tags` is already covered in Story 17.1. This row exists because the original audit counted it separately but it's the same test.

**Total from connection_tests.rs:** 7 tests (indices 311, 323, 338, 349, 375, 467, 554)

---

### Story 17.4: Wire connection error/refused test to may runtime

**Files changed:** `src/connection/tcp_tests.rs`

| # | Test | What it validates | Conversion |
|---|------|-------------------|------------|
| 9 | `test_connect_refused_returns_connect` | Connecting to port 1 returns `ConnectionError::Connect` | Remove `#[ignore]`, wrap in `may::run` + `may::go!` |

**Correction:** The test already connects to `127.0.0.1:1` (TCP port 1, which is reserved and always refuses). It asserts `result.is_err()` and checks the error type. It does NOT need a live server, Docker, or any random port — port 1 will always be unreachable. The ignore tag is wrong. It just needs may runtime wrapping.

**Conversion (remove ignore, wrap in may runtime):**
```rust
#[test]
fn test_connect_refused_returns_connect() {
    use may::go;

    let wrapper = std::sync::Mutex::new(None::<()>);
    let _wrapper2 = wrapper.lock().unwrap();
    let wrapper2 = std::sync::Arc::new(std::sync::Mutex::new(None::<()>));
    let wrapper3 = std::sync::Arc::clone(&wrapper2);

    may::run(|| {
        may::go(move || {
            let result = TcpConnector::connect_timeout("127.0.0.1", 1, 5);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ConnectionError::Connect(_)));
            *wrapper3.lock().unwrap() = Some(());
        }).join();
    });
}
```

---

### Story 17.5: Wire resource limit tests to may runtime

**Files changed:** `src/connection/test_limits.rs`

| # | Test | What it validates | Conversion |
|---|------|-------------------|------------|
| 10 | `test_connect_with_limits_custom_depth` | `connect_with_limits()` accepts custom params without panic | Remove `#[ignore]`, wrap in `may::run` + `may::go!` |
| 11 | `test_connect_with_limits_large_request_size` | `connect_with_limits()` accepts large params without panic | Remove `#[ignore]`, wrap in `may::run` + `may::go!` |
| 12 | `test_queue_full_returns_error` | Queue depth limit triggers QueueFull error | Use Docker fixture port |
| 13 | `test_request_too_large_returns_error` | Request size limit triggers RequestTooLarge error | Use Docker fixture port |

**Critical correction for 10-11:** These tests call `connect_with_limits("127.0.0.1", 6379, ...)` and assert `result.is_err()`. They **expect the connection to fail** — their purpose is to verify the parameter values are accepted without panic, not to test networking. The `#[ignore = "requires live network namespace"]` tag is **wrong**. They need may runtime (because `spawn_connection_loop` runs inside `connect_with_limits`), but they do NOT need a live server.

**Conversion for 10-11 (remove ignore, wrap in may runtime):**
```rust
#[test]
fn test_connect_with_limits_custom_depth() {
    may::run(|| {
        may::go(|| {
            let result = Connection::connect_with_limits(
                "127.0.0.1",
                6379,
                std::time::Duration::from_secs(1),
                10,   // custom queue depth
                1024, // custom request size
            );
            assert!(result.is_err()); // no Redis — connection expected to fail
        }).join();
    });
}
```

**Conversion for 12-13 (use Docker fixture — these need a live connection to exercise limits):**
```rust
#[test]
fn test_queue_full_returns_error() {
    if test_fixture::skip_docker_tests() {
        return;
    }
    let port = test_fixture::plain_redis_port().expect("fixture port");
    let Ok(conn) = Connection::connect_with_limits(
        "127.0.0.1",
        port,
        std::time::Duration::from_secs(1),
        2,    // tiny queue depth
        65536,
    ) else {
        return;
    };
    // ... fill queue, assert QueueFull on 3rd send ...
}
```

---

### Story 17.6: Fix broken doctest `ignore` in cluster module

**Files changed:** `src/cluster/cluster_client.rs`

**Current (line 9):**
```rust
/// ```ignore
/// use may_redis::cluster::cluster_client::RedisClusterClient;
/// ```
```

**Problem:** `RedisClusterClient` doesn't exist. The type is `ClusterClient`.

**Fix:** Change `ignore` to a regular code block, reference the correct type:
```rust
/// ```
/// use may_redis::cluster::cluster_client::ClusterClient;
/// // ... actual example code ...
/// ```
```

**If no working example exists:** Either:
1. Remove the `/// ```ignore` and replace with a working example using `ClusterClient`, or
2. If there's no sensible example (because ClusterClient requires multi-node setup not feasible in doctests), add a doc comment explaining it instead and remove the code block entirely

---

## Test Coverage Matrix

### Before (13 ignored)

| Location | Ignored Tests | Category |
|----------|---------------|----------|
| `connection_tests.rs` | 7 | Connection lifecycle + drop/cancellation regression |
| `tcp_tests.rs` | 1 | Error handling (connect refused) |
| `test_limits.rs` | 4 | Resource limits (queue, request size) |
| `cluster/cluster_client.rs` | 1 | Broken doctest example |
| **Total** | **13** | |

### After (0 ignored)

| Story | Tests Unignored | New Tests Added |
|-------|-----------------|-----------------|
| 17.1 | 4 (connect, send_tags, drop, id) | 0 |
| 17.2 | 3 (drop_during_pipeline, drop_during_request, drop_no_panic) | 0 |
| 17.3 | 0 | — |
| 17.4 | 1 (connect_refused) | 0 |
| 17.5 | 4 (limits_custom_depth, limits_large_request, queue_full, request_too_large) | 0 |
| 17.6 | 1 (cluster_client doctest) | 0 |
| **Total** | **13** | **0** |

All 13 existing tests are unignored. No new tests are added. The work is **conversion only** — wiring existing tests to the fixture infrastructure.

---

## Verification

### Unit test verification
```bash
# Run the previously-ignored tests individually
cargo test --workspace --features test test_connection_connect -- --nocapture
cargo test --workspace --features test test_connection_send_tags -- --nocapture
cargo test --workspace --features test test_connection_drop -- --nocapture
cargo test --workspace --features test test_connection_id -- --nocapture
cargo test --workspace --features test test_connection_drop_during_pipeline -- --nocapture
cargo test --workspace --features test test_connection_drop_during_request -- --nocapture
cargo test --workspace --features test test_connection_drop_no_panic -- --nocapture
cargo test --workspace --features test test_connect_refused_returns_connect -- --nocapture
cargo test --workspace --features test test_connect_with_limits_custom_depth -- --nocapture
cargo test --workspace --features test test_connect_with_limits_large_request_size -- --nocapture
cargo test --workspace --features test test_queue_full_returns_error -- --nocapture
cargo test --workspace --features test test_request_too_large_returns_error -- --nocapture
cargo test --doc cluster/cluster_client -- test_cluster_refcell --nocapture
```

Each must: (a) run instead of being ignored, (b) pass when Docker is available, (c) skip gracefully when Docker is not available.

### Full suite verification
```bash
# Zero ignored tests across all suites
cargo test --workspace --features test -- --list 2>&1 | grep -c "ignored"
# Expected: 0

# Full suite passes
cargo test --workspace --features test
# Expected: 620 passed, 0 failed, 0 ignored, 0 measured
```

### CI verification
After this PR merges, CI should show:
- 620 passed, 0 failed, **0 ignored** (was 13 ignored)
- No `ignored` in any test output
- No `--- [IGNORED] ---` markers in CI logs

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Docker not available in some CI runners | Low | `skip_docker_tests()` already handles this — tests skip gracefully |
| Test flakiness from container startup timing | Low | Docker fixture already has `ContainerNotReady` timeout handling; 100+ existing integration tests prove stability |
| `test_connect_refused` needs a port that nothing listens on | Low | Use `127.0.0.1:1` or a well-known ephemeral range (50000-59999) — unlikely to be in use |
| Resource limit tests need real connection to fill queue | Low | Docker fixture provides real connection; queue fill logic is deterministic (2 sends fills queue of depth 2) |
| Doctest fix may need updated example code | Low | If `ClusterClient` example doesn't exist, replace with doc-only comment (no code block) |

---

## Implementation Order

1. **Story 17.6** — Fix broken doctest (trivial, no infrastructure needed). Can do first to validate approach.
2. **Story 17.1** — Wire 4 basic connection lifecycle tests.
3. **Story 17.2** — Wire 3 critical drop/cancellation regression tests.
4. **Story 17.3** — (Merged with 17.1 — no separate work.)
5. **Story 17.4** — Wire connect refused test.
6. **Story 17.5** — Wire 4 resource limit tests.

Stories 17.1 and 17.2 share the same file and pattern, so they can be done in a single commit. Stories 17.4 and 17.5 also share the `skip_docker_tests()` pattern.

**Recommended: Single commit for stories 17.1-17.5, separate commit for 17.6.**

---

## Success Criteria

- [ ] Zero ignored tests across all suites (unit, integration, doctests, perf)
- [ ] All 13 tests run and pass when Docker is available
- [ ] All 13 tests skip gracefully (not fail) when Docker is unavailable
- [ ] `cargo test --workspace --features test` reports `0 ignored`
- [ ] CI passes with 620 passed, 0 failed, **0 ignored**
- [ ] No new test failures introduced in other suites
- [ ] No regression in existing 620 passing tests
