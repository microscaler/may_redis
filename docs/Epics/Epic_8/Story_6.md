# Story 8.6 — FromRedisValue for () Must Accept Integer(0)

**Objective:** Fix `FromRedisValue for ()` to accept `RedisValue::Integer(0)` in addition to `Integer(1)` and `SimpleString("OK")`. Redis `SETNX` returns `Integer(0)` when the key already exists (not-set case). The current implementation rejects `Integer(0)` with a `Parse` error, breaking the typed API for `SETNX` and similar commands that use `0` to indicate "not performed."

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1 (basic FromRedisValue impls present).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #2, CRITICAL), `src/core/error.rs` lines 103-112

## The Bug

```rust
impl FromRedisValue for () {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::SimpleString(s) if s == "OK" => Ok(()),
            RedisValue::Integer(1) => Ok(()),
            other => Err(RedisError::Parse(format!(
                "expected OK or Integer(1), got {other:?}"
            ))),
        }
    }
}
```

`Integer(0)` falls into the `other` arm, producing `Parse: "expected OK or Integer(1), got Integer(0)"`.

**Impact:** `client.execute::<()>(client.setnx("existing_key", "value"))` where key exists → `Err(Parse(...))` instead of `Ok(())`.

## Functional Requirements

1. `FromRedisValue for ()` must accept:
   - `SimpleString("OK")` → `Ok(())` (SET, DEL, etc.)
   - `Integer(1)` → `Ok(())` (SETNX success, etc.)
   - `Integer(0)` → `Ok(())` (SETNX failure, etc.)
2. `FromRedisValue for ()` must reject:
   - `Error` → `Err(Parse(...))` with descriptive message
   - `Null` → `Err(Parse(...))` with descriptive message
   - `BulkString` → `Err(Parse(...))` with descriptive message
   - `Array` → `Err(Parse(...))` with descriptive message

## Non-Functional Requirements

1. **Backwards compatible** — existing callers using `()` for SET/DEL/etc. see no change.
2. **Zero may dependency** — `error.rs` has no `may` imports.
3. **Error messages** — must distinguish `Integer(0)` rejection from other rejections.

## Code Anchors

- `src/core/error.rs` lines 103-112 — `impl FromRedisValue for ()`

## Tasks

1. Add `RedisValue::Integer(0) => Ok(())` to the match arm in `FromRedisValue for ()`.
2. Update the error message to reflect accepted values: "expected OK, Integer(0), or Integer(1)".
3. Write unit tests for `Integer(0)` acceptance and `Error`/`Null` rejection.
4. Verify `Option<String>` still delegates correctly (it has its own impl in `from_value.rs`).

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `unit_to_unit_ok` | `SimpleString("OK")` | `Ok(())` |
| `unit_to_unit_int1` | `Integer(1)` | `Ok(())` |
| `unit_to_unit_int0` | `Integer(0)` | `Ok(())` |
| `unit_to_unit_null_error` | `Null` | `Err(Parse)` |
| `unit_to_unit_error_val` | `Error("ERR msg")` | `Err(Parse)` |
| `unit_to_unit_bulk_error` | `BulkString(b"nope")` | `Err(Parse)` |
| `unit_to_unit_array_error` | `Array([...])` | `Err(Parse)` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238 tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `client.execute::<()>(client.set("k","v"))` → `Ok(())` (no regression)
- [ ] `client.execute::<()>(client.setnx("k","v"))` → `Ok(())` for both existing and new keys
- [ ] `() as FromRedisValue` does not accept `Error`, `Null`, `BulkString`, or `Array`
