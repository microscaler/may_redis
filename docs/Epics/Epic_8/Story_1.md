# Story 8.1 — FromRedisValue for Basic Types

**Objective:** Implement `FromRedisValue` for the four most-used basic response types: `String`, `i64`, `bool`, and `()`. These are the building blocks that make the typed API usable.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Epic 7 (all 122 command methods complete, 122 tests passing, clippy clean).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #1, critical), `docs/01-protocol-analysis.md`

## Functional Requirements

1. `String` converts from `RedisValue::BulkString`, `RedisValue::SimpleString`, and `RedisValue::Integer` (via numeric string representation). Returns `RedisError::Parse` for other types.
2. `i64` converts from `RedisValue::Integer`. Returns `RedisError::Parse` for other types.
3. `bool` converts from `RedisValue::Integer(1)` → `true`, `RedisValue::Integer(0)` → `false`. Returns `RedisError::Parse` for other types.
4. `()` converts from `RedisValue::SimpleString` (e.g., "OK" from SET), `RedisValue::Integer` (e.g., ":1" from SET). Returns `RedisError::Parse` for `RedisValue::Error`. For `RedisValue::BulkString` and `RedisValue::Array`, also returns `()`. Returns `RedisError::Parse` for `RedisValue::Null` (null is unexpected for unit return types).

## Non-Functional Requirements

1. **Zero may dependency** — `from_value.rs` has no `may` imports.
2. **Coherence safe** — `impl FromRedisValue for String` and `impl FromRedisValue for Option<String>` must not conflict. `Option<T>` delegates to `T::from_redis_value` on Non-Null, returns `Ok(None)` on Null.
3. **Error messages** — `RedisError::Parse` messages must be descriptive: `"expected Integer, got BulkString"`.
4. **Backwards compatible** — Existing `FromRedisValue` impls (`Vec<String>`, `Vec<i64>`, `Vec<RedisValue>`, `Option<String>`, `usize`) are unchanged.

## Code Anchors

- `src/core/from_value.rs` — `impl FromRedisValue for String`, `impl FromRedisValue for i64`, `impl FromRedisValue for bool`, `impl FromRedisValue for ()`

## Tasks

1. Implement `FromRedisValue for String` — handles BulkString, SimpleString, Integer
2. Implement `FromRedisValue for i64` — handles Integer only
3. Implement `FromRedisValue for bool` — handles Integer(0) and Integer(1)
4. Implement `FromRedisValue for ()` — handles SimpleString, Integer, BulkString, Array; rejects Error and Null
5. Verify `Option<String>` delegation still works (existing impl)
6. Verify `Option<i64>` delegation works with new i64 impl
7. Write unit tests for each impl

## Unit Test Plan

Each impl gets 4 tests:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `from_redis_value_string_bulk` | BulkString("hello") | Ok("hello".to_string()) |
| `from_redis_value_string_simple` | SimpleString("OK") | Ok("OK".to_string()) |
| `from_redis_value_string_integer` | Integer(42) | Ok("42".to_string()) |
| `from_redis_value_string_error` | Error("ERR msg") | Err(Parse) |
| `from_redis_value_i64_integer` | Integer(42) | Ok(42i64) |
| `from_redis_value_i64_negative` | Integer(-1) | Ok(-1i64) |
| `from_redis_value_i64_string` | BulkString("42") | Err(Parse) |
| `from_redis_value_i64_null` | Null | Err(Parse) |
| `from_redis_value_bool_true` | Integer(1) | Ok(true) |
| `from_redis_value_bool_false` | Integer(0) | Ok(false) |
| `from_redis_value_bool_other` | Integer(42) | Err(Parse) |
| `from_redis_value_unit_simple` | SimpleString("OK") | Ok(()) |
| `from_redis_value_unit_integer` | Integer(1) | Ok(()) |
| `from_redis_value_unit_null` | Null | Err(Parse) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 262 tests pass (259 existing + 13 new)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] New `FromRedisValue` for `i64` does not conflict with existing `usize` impl
- [ ] `Option<String>` and `Option<i64>` still work with new base impls
- [ ] Each command method from Epic 7 can be called with basic return types (e.g., `client.execute::<String>(client.get("key"))`)
