# Story 8.9 — Add Command Execution Timeout

**Objective:** Add a configurable timeout for command execution. Currently, if the connection loop hangs (epoll bug, dead socket, blocked I/O), `RedisClient::execute()` blocks forever on `rx.recv()`. This story adds a timeout that cancels waiting coroutines after a configurable duration.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.3 (connection timeout infrastructure).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #5, HIGH), `src/client/client.rs` lines 70-88, `may` crate timer docs

## The Problem

The connection layer (Story 8.3) added a timeout for TCP connect, but command execution has no timeout. The `execute()` method:

```rust
let response = rx.recv()
    .map_err(|_| RedisError::Parse("response channel closed".into()))?;
```

If the connection loop never dispatches a response (e.g., epoll bug, dead TCP socket, server crash), this blocks forever. The coroutine is stuck in `spsc::Receiver::recv()` with no escape hatch.

## Functional Requirements

1. `RedisClient::execute_with_timeout<T>(cmd, timeout)` — executes command with a `Duration` timeout. If the response is not received within `timeout`, returns `Err(RedisError::Connection(format!("command execution timed out after {:?}")))`.
2. `RedisClient::execute_timeout<T>(cmd, seconds: u32)` — convenience wrapper using `Duration::from_secs`.
3. Existing `RedisClient::execute<T>(cmd)` gets a default timeout of 30 seconds.
4. Timeout must use `may::timer::sleep()` in a spawned coroutine that races against `rx.recv()`.
5. If the connect completes first, cancel the timer coroutine (it exits and drops its receiver).
6. If the timer fires first, drop the `rx` (the `recv()` call will fail with channel closed) and return timeout error.
7. The timeout applies only to the command execution wait, not to DNS resolution or TCP connect.

## Non-Functional Requirements

1. **Zero new dependencies** — uses `may::timer::sleep` (already available via `may` crate).
2. **No may dependency in protocol** — timeout logic lives in `client/`, not `protocol/`.
3. **Clean cancellation** — if connect completes before timeout, the timer coroutine must not hold any resources (no channel leaks).
4. **Backwards compatible** — existing `execute()` calls get a 30s default; API users can opt into shorter timeouts.

## Code Anchors

- `src/client/client.rs` — `execute_with_timeout()`, `execute_timeout()` methods
- `src/core/error.rs` — possibly add `ExecutionTimeout` variant to `RedisError` (or reuse `Connection`)

## Tasks

1. Add `Timeout` variant to `RedisError` (or add `is_timeout()` method on existing `Connection` variant).
2. Implement `execute_with_timeout()` in `RedisClient` — spawns timer coroutine, races with `rx.recv()`.
3. Implement `execute_timeout()` convenience wrapper.
4. Update `execute()` to call `execute_with_timeout()` with 30s default.
5. Write unit tests for timeout and non-timeout paths.

## Unit Test Plan

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `timeout_fast_response` | Command completes in < 1ms | `Ok(response)` (no timeout) |
| `timeout_slow_response` | Fake scenario: no response within timeout | `Err(Timeout)` |
| `timeout_cancel_on_response` | Response arrives before timeout fires | Timer coroutine cleaned up, no leak |
| `timeout_zero` | Timeout = Duration::ZERO | Must NOT immediately time out; may need 1ms minimum |
| `timeout_default_30s` | `execute()` with no explicit timeout | Uses 30s default |

**Note:** The slow-response test requires mocking the connection loop or using a real Redis server with a slow command (e.g., `SLEEP 10`). These tests should be `#[ignore]`.

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass (timeout tests are `#[ignore]`)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `execute()` completes normally when Redis responds within 30s
- [ ] `execute_with_timeout(cmd, 1s)` returns `Err` when response takes > 1s
- [ ] Timer coroutine does not leak resources when response arrives first
- [ ] No panic on timeout (clean error propagation)
