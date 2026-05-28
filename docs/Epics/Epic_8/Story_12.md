# Story 8.12 — Add Unit Test for i64::from_redis_value Rejection of BulkString

**Objective:** Add comprehensive unit tests for `FromRedisValue for i64` to verify that it correctly rejects all non-Integer types, especially `BulkString` representations of numbers. This is a coverage gap — the implementation exists but has no test for `BulkString("42")` → error.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure test addition).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #8, HIGH — coverage gap), `src/core/error.rs` lines 78-87

## The Gap

The `FromRedisValue for i64` impl only accepts `Integer`:

```rust
impl FromRedisValue for i64 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) => Ok(*n),
            other => Err(RedisError::Parse(format!(
                "expected Integer, got {other:?}"
            ))),
        }
    }
}
```

Existing tests in `error.rs` cover:
- `Integer(42)` → `Ok(42)` ✓
- `BulkString("not an int")` → `Err` ✓ (but this tests wrong-type, not numeric-string)

Missing:
- `BulkString("42")` → must return `Err` (numeric string should NOT be auto-converted)
- `SimpleString("42")` → must return `Err` (numeric simple string)
- `Null` → must return `Err`
- `Integer(0)` → must return `Ok(0)` (edge case)
- `Integer(-9223372036854775808)` → `Ok(i64::MIN)` (overflow edge case)
- `Integer(9223372036854775807)` → `Ok(i64::MAX)` (max edge case)

**Why it matters:** If someone stores `"42"` as a string key in Redis and calls `execute::<i64>(client.get("key"))`, the response is `BulkString(b"42")`. This should fail — the user asked for an integer, got a string. Silently converting numeric strings would be wrong and could hide type bugs.

## Functional Requirements

1. Add tests for `BulkString("42")` → `Err(Parse)` (numeric string rejection).
2. Add tests for `SimpleString("42")` → `Err(Parse)` (numeric simple string rejection).
3. Add tests for `Null` → `Err(Parse)`.
4. Add tests for `Integer(0)`, `Integer(i64::MIN)`, `Integer(i64::MAX)`.
5. Add tests for `Error("ERR msg")` → `Err(Parse)`.
6. Add tests for `Array([Integer(1)])` → `Err(Parse)`.

## Non-Functional Requirements

1. **Zero may dependency** — tests in `error.rs`.
2. **Descriptive error messages** — each test verifies the error type matches.
3. **No code changes** — this story is test-only.

## Code Anchors

- `src/core/error.rs` lines 78-87 — `impl FromRedisValue for i64`
- `src/core/error.rs` lines 128-207 — existing tests in `mod tests`

## Tasks

1. Add 8+ new test cases to the `mod tests` in `error.rs`.
2. Verify error messages are descriptive.
3. Run `cargo test` to confirm all pass.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `i64_from_bulk_numeric` | `BulkString(b"42")` | `Err(Parse)` |
| `i64_from_simple_numeric` | `SimpleString("42")` | `Err(Parse)` |
| `i64_from_null` | `Null` | `Err(Parse)` |
| `i64_from_error_val` | `Error("ERR")` | `Err(Parse)` |
| `i64_from_array` | `Array([Integer(1)])` | `Err(Parse)` |
| `i64_from_zero` | `Integer(0)` | `Ok(0)` |
| `i64_from_min` | `Integer(i64::MIN)` | `Ok(-9223372036854775808)` |
| `i64_from_max` | `Integer(i64::MAX)` | `Ok(9223372036854775807)` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass (including new ones)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `execute::<i64>(get("string_key"))` → `Err(Parse)` (string "42" not auto-converted)
- [ ] `execute::<i64>(incr("counter"))` → `Ok(1)` (integer responses work)
