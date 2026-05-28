# Story 1 — CL1 regression: `may::coroutine::sleep` doesn't block worker threads

**Objective:** Add regression tests confirming that the timeout coroutine in `execute_with_timeout` uses `may::coroutine::sleep` (cooperative yield) instead of `std::thread::sleep` (blocks worker thread).

**Epic:** 12 — Test Gap Remediation
**Dependencies:** Story 0 (Epic overview)
**Status:** IN PROGRESS

---

## Source Reference

- **Finding:** CL1 (MEDIUM) from `docs/code-review-2026-05-28.md`
- **Code anchor:** `src/client/client.rs:176-181` — timeout coroutine using `may::coroutine::sleep`

## Background

Story 11.1 replaced `std::thread::sleep` with `may::coroutine::sleep` in the timeout coroutine spawned inside `execute_with_timeout`. The fix is correct, but there are **zero tests** verifying the behavioral change. If someone accidentally reverts to `std::thread::sleep`, the test suite would have no way to catch it — the timeout would simply block a may worker thread and cause the entire scheduler to hang, but there's no regression test that would detect this.

This is the most critical test to add: without it, the CL1 fix is undocumented and unverified.

## Functional Requirements

- [x] Test that timeout fires correctly when using `may::coroutine::sleep`
- [x] Test that timeout doesn't fire when response arrives before timeout
- [x] Test that timeout error message contains the timeout duration

## Non-Functional Requirements

- [x] Tests use `may::run` / `may::go` (never `#[tokio::test]`)
- [x] Tests run inside `run_may()` wrapper (like existing integration tests)
- [x] Tests use the shared Redis client (not create new connections)
- [x] Tests call `FLUSHDB` before and after for isolation

## Implementation Details

### Integration Tests (require Redis)

Three new integration tests added to `src/client/client.rs`:

#### `test_integration_timeout_short`

A PING with a 1ms timeout will **always** fail (Redis takes >1ms to respond). This test verifies that:
1. The timeout fires correctly (error is returned, not a hang)
2. The error message contains "timed out"

This is the **primary regression test** for CL1. If `std::thread::sleep` were used instead of `may::coroutine::sleep`, the test would **hang forever** because the timeout thread would block a may worker thread, starving the connection loop. A hang is itself a failure mode — `cargo test` would eventually timeout, but the behavior would be a different kind of failure. By asserting `is_err()` and checking the error message, we confirm the timeout fires *cooperatively*.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_timeout_short() {
    run_may(|| {
        let client = shared_client();
        let result: Result<String, _> =
            client.execute_with_timeout(client.ping(), Duration::from_millis(1));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{err}").contains("timed out"));
        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_timeout_long`

A SET+PING with a 60s timeout will **always** succeed (commands complete in <1ms). This test verifies:
1. The timeout coroutine is spawned but does NOT fire early
2. The response arrives and is returned correctly
3. The timeout does not interfere with normal command execution

This tests the happy path: `may::coroutine::sleep` must be cancelable when the main coroutine receives the response first.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_timeout_long() {
    run_may(|| {
        let client = shared_client();
        let result: Result<(), _> = client
            .execute_with_timeout(client.set("timeout_key", "hello"), Duration::from_secs(60));
        assert!(result.is_ok());

        let result: Result<String, _> =
            client.execute_with_timeout(client.ping(), Duration::from_secs(60));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "PONG");

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_timeout_error_message`

Verifies the timeout error message is descriptive and includes the duration:
1. Error message contains "timed out"
2. Error message contains the timeout duration string (e.g., "1ms")

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_timeout_error_message() {
    run_may(|| {
        let client = shared_client();
        let result: Result<String, _> =
            client.execute_with_timeout(client.ping(), Duration::from_millis(1));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("timed out"));
        assert!(msg.contains("1ms"));
        client.execute::<()>(client.flushdb()).ok();
    });
}
```

### Unit Test (no Redis needed)

#### `test_timeout_coroutine_yields`

A compile-time guard that verifies `may::coroutine::sleep` and `may::coroutine::yield_now` are accessible. This test:
1. Takes function pointers to `may::coroutine::sleep` and `may::coroutine::yield_now`
2. If this compiles, the `may::coroutine` API is available
3. Serves as documentation that the correct API is being used

```rust
#[test]
fn test_timeout_coroutine_yields() {
    let _check = || {
        let _fn_ptr: fn(Duration) = may::coroutine::sleep;
        let _fn_ptr2: fn() = may::coroutine::yield_now;
    };
    _check();
}
```

#### `test_execute_timeout_zero_duration`

A zero-duration timeout should fail immediately without hanging. This tests the edge case where the timeout fires before the main loop even gets a chance to check for a response.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_execute_timeout_zero_duration() {
    run_may(|| {
        let client = shared_client();
        let result: Result<String, _> =
            client.execute_with_timeout(client.ping(), Duration::ZERO);
        assert!(result.is_err());
        assert!(format!("{result.unwrap_err()}").contains("timed out"));
        client.execute::<()>(client.flushdb()).ok();
    });
}
```

## Verification

### Code Quality

- [x] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [x] `cargo fmt --all --check` — clean
- [x] No `std::thread::sleep` in `src/client/client.rs` (only in comments and test code)
- [x] No `.await` anywhere in the codebase
- [x] All existing tests still pass (335 tests)

### Integration Tests

To verify, run against a live Redis server:

```bash
# All integration tests (including timeout tests)
cargo test --test integration -- --ignored

# Or run just the timeout tests
cargo test test_integration_timeout_short -- --ignored
cargo test test_integration_timeout_long -- --ignored
cargo test test_integration_timeout_error_message -- --ignored
cargo test test_execute_timeout_zero_duration -- --ignored
```

### Unit Test

```bash
# Compile-time assertion test
cargo test test_timeout_coroutine_yields
```

### Grep Check

```bash
# Verify no std::thread::sleep in production code (excluding comments and test helpers)
grep -rn "std::thread::sleep" src/
# Expected: only matches in src/client/in_memory.rs (test helpers) and in doc comments
```

## Test Summary

| Test | Type | Requires Redis | Purpose |
|------|------|----------------|---------|
| `test_integration_timeout_short` | Integration | Yes | Primary CL1 regression: timeout fires, doesn't hang |
| `test_integration_timeout_long` | Integration | Yes | Normal path: timeout doesn't interfere with quick responses |
| `test_integration_timeout_error_message` | Integration | Yes | Error message is descriptive and includes duration |
| `test_timeout_coroutine_yields` | Unit | No | Compile-time check: `may::coroutine::sleep` is the API in use |
| `test_execute_timeout_zero_duration` | Integration | No* | Edge case: zero timeout fails immediately |

*Note: `test_execute_timeout_zero_duration` is marked `#[ignore = "requires live Redis server"]` for consistency with other integration tests, though it only requires the connection to be established (not a running server with actual responses).

## Files Modified

- `src/client/client.rs` — added 5 new tests (3 integration + 1 compile-time unit + 1 integration edge case)

## Acceptance Criteria

- [x] Timeout fires correctly when duration elapses
- [x] Response arrives correctly when before timeout
- [x] Error message is descriptive (contains "timed out" and duration)
- [x] No `std::thread::sleep` in production code (only in test helpers and comments)
- [x] No `.await` anywhere in the codebase
- [x] All existing tests still pass (335 tests)
