# Story 8.8 — Redis Error Responses Must Be Preserved in Client.execute()

**Objective:** Fix `RedisClient::execute()` to detect when the response is a `RedisValue::Error` and return it as a `RedisError::Protocol` with the original Redis error message intact. Currently, the error response is passed directly to `from_redis_value()`, which treats it as a type mismatch and returns a generic `Parse` error, losing the original Redis error message.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1-8.7 (type conversions correct).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #4, CRITICAL), `src/client/client.rs` lines 70-88

## The Bug

```rust
pub fn execute<T: FromRedisValue>(&self, cmd: CommandBuilder) -> Result<T, RedisError> {
    let data = cmd.build();
    let (tx, rx) = spsc::channel();
    let request = Request::new(data.to_vec(), tx);
    let _tag = self.inner.connection.send(request);
    let response = rx.recv()...;
    T::from_redis_value(&response)  // <-- RedisValue::Error passed directly
}
```

When Redis returns `-ERR wrong type of key`, the response is `RedisValue::Error("ERR wrong type of key")`. This is passed to `from_redis_value::<String>(...)`, which sees the `Error` variant and returns `Parse: "expected BulkString or SimpleString, got Error(...)"`.

**Impact:** Users cannot distinguish between a Redis protocol error and a Rust-side parse error. The original Redis error message is buried inside a `Parse` wrapper.

## Functional Requirements

1. `execute()` must check if `response` is `RedisValue::Error` before calling `from_redis_value()`.
2. If `response` is `Error(msg)`, return `Err(RedisError::Protocol(msg))` — preserving the original message.
3. For all other response types, delegate to `T::from_redis_value(&response)` as before.
4. The same check must apply to `Pipeline::execute_raw()` — individual error responses must be wrapped in `RedisValue::Error`, not silently converted.
5. `Pipeline::execute<T: FromPipelineResponse>()` must propagate `RedisValue::Error` as `RedisError::Protocol`.

## Non-Functional Requirements

1. **Backwards compatible** — callers who were catching `Parse` errors from Redis errors will see `Protocol` instead. This is a behavioral change but correct — it was always a protocol-level issue.
2. **Zero may dependency** — client.rs already imports `may`, no new dependencies.
3. **Error type** — use `RedisError::Protocol` (not `Other`) for Redis protocol errors.

## Code Anchors

- `src/client/client.rs` lines 70-88 — `RedisClient::execute()`
- `src/client/pipeline.rs` lines 80-113 — `Pipeline::execute_raw()` and `execute()`

## Tasks

1. In `RedisClient::execute()`, add a check: if `response == RedisValue::Error(msg)`, return `Err(RedisError::Protocol(msg))`.
2. In `Pipeline::execute_raw()`, wrap error responses in `RedisValue::Error` as-is (they already are), but ensure `FromPipelineResponse` implementations handle them.
3. Update `FromPipelineResponse` implementations to propagate `RedisValue::Error` as `RedisError::Protocol`.
4. Write unit tests for Redis error preservation.
5. Write integration tests with live Redis (ignorable) to verify error messages.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `execute_redis_error_preserved` | Response = `Error("WRONGTYPE")` | `Err(Protocol("WRONGTYPE"))` |
| `execute_redis_error_long_msg` | Response = `Error("ERR value is not an integer or out of range")` | `Err(Protocol("ERR value is not an integer or out of range"))` |
| `execute_success_after_error` | Response = `SimpleString("OK")` | `Ok(())` (no regression) |
| `pipeline_error_preserved` | Pipeline response includes `Error("WRONGTYPE")` | `Err(Protocol("WRONGTYPE"))` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238 tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `client.execute::<String>(cmd)` with an error response → `Err(Protocol("original message"))`
- [ ] `client.execute::<String>(cmd)` with a success response → `Ok("response")` (no regression)
- [ ] Pipeline error handling propagates Redis error messages
- [ ] Error message is the raw Redis message (no "expected X, got Error" wrapping)
