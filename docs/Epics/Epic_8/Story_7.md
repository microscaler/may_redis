# Story 8.7 — FromRedisValue for String Must Accept Integer

**Objective:** Add `Integer` support to `FromRedisValue for String`. When a Redis response is an integer (e.g., `Integer(42)`), converting to `String` should produce `"42"`. This is needed for commands where the Redis server might return an integer that the user wants as a string representation.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1 (basic types). Story 8.6 must be complete (no conflicts).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #3, CRITICAL — Story 8.1 spec says String should accept Integer), `src/core/error.rs` lines 89-100

## The Bug

Story 8.1 spec says: `String` converts from `BulkString`, `SimpleString`, AND `Integer` (via numeric string representation). The actual implementation only handles `BulkString` and `SimpleString`.

```rust
impl FromRedisValue for String {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::BulkString(bytes) => ...,    // works
            RedisValue::SimpleString(s) => ...,      // works
            other => Err(...),                       // Integer(42) → error!
        }
    }
}
```

**Impact:** `client.execute::<String>(client.dbsize())` fails with `Parse: "expected BulkString or SimpleString, got Integer(0)"` instead of `Ok("0")`.

## Functional Requirements

1. `FromRedisValue for String` must accept:
   - `BulkString(bytes)` → convert UTF-8 bytes to String
   - `SimpleString(s)` → return `s` directly
   - `Integer(n)` → format as `n.to_string()` (e.g., `Integer(42)` → `"42"`)
2. Must reject:
   - `Error` → `Err(Parse("cannot convert Error to String"))`
   - `Null` → `Err(Parse("cannot convert Null to String"))`
   - `Array` → `Err(Parse("cannot convert Array to String"))`

## Non-Functional Requirements

1. **Backwards compatible** — callers using `String` with BulkString/SimpleString responses see no change.
2. **Zero may dependency** — `error.rs` has no `may` imports.
3. **Descriptive errors** — must clearly state which type was received.

## Code Anchors

- `src/core/error.rs` lines 89-100 — `impl FromRedisValue for String`

## Tasks

1. Add `RedisValue::Integer(n) => Ok(n.to_string())` to the match arm.
2. Update error message to include "or Integer" in the "expected" clause.
3. Write unit tests for `Integer` acceptance and `Error`/`Null`/`Array` rejection.
4. Run integration tests to verify no regressions in typed command usage.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `string_from_int_42` | `Integer(42)` | `Ok("42")` |
| `string_from_int_negative` | `Integer(-1)` | `Ok("-1")` |
| `string_from_int_zero` | `Integer(0)` | `Ok("0")` |
| `string_from_int_max` | `Integer(i64::MAX)` | `Ok("9223372036854775807")` |
| `string_from_bulk` | `BulkString(b"hello")` | `Ok("hello")` |
| `string_from_simple` | `SimpleString("OK")` | `Ok("OK")` |
| `string_from_error` | `Error("ERR msg")` | `Err(Parse)` |
| `string_from_null` | `Null` | `Err(Parse)` |
| `string_from_array` | `Array([...])` | `Err(Parse)` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238 tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `client.execute::<String>(client.dbsize())` → `Ok("0")` when DB is empty
- [ ] `client.execute::<String>(client.incr("x"))` → `Ok("1")` when x did not exist
- [ ] No regression in `client.execute::<String>(client.get("key"))` with string values
