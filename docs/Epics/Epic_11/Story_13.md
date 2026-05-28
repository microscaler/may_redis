# Story 11.13 — Add `FromRedisValue` impls for common integer types

**Objective:** Add `FromRedisValue` implementations for `u64`, `i32`, `u8`, and `f64` to support the expanding command surface (e.g., INCRBYFLOAT).

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding T2, INFO)

**Finding:** T2 — No `FromRedisValue` impl for `u64`, `i32`, `u8`, or `f64`. These may be needed as the command surface expands (e.g., INCRBYFLOAT).

## Functional Requirements

- [ ] Implement `FromRedisValue for u64` — parse from `RedisValue::Integer`, handle overflow
- [ ] Implement `FromRedisValue for i32` — parse from `RedisValue::Integer`, handle overflow
- [ ] Implement `FromRedisValue for u8` — parse from `RedisValue::Integer`, handle overflow
- [ ] Implement `FromRedisValue for f64` — parse from `RedisValue::BulkString` (Redis returns doubles as bulk strings)
- [ ] All implementations must return `RedisError::Parse` on type mismatch

## Non-Functional Requirements

- [ ] Follow the existing pattern: match the `RedisValue` variant, convert, return error on mismatch
- [ ] Integer impls: check range fits in target type before casting
- [ ] `f64` impl: parse from the bulk string bytes using `f64::from_str`

## Code Anchors

- `src/core/from_value.rs` — Where existing `FromRedisValue` impls live

## Tasks

1. Add `FromRedisValue for u64`:
   - Match `RedisValue::Integer(n)`, check `n >= 0 && n <= u64::MAX`
   - Return `Err(RedisError::Parse("value out of range for u64"))` on overflow
2. Add `FromRedisValue for i32`:
   - Match `RedisValue::Integer(n)`, check `n >= i32::MIN as i64 && n <= i32::MAX as i64`
3. Add `FromRedisValue for u8`:
   - Match `RedisValue::Integer(n)`, check `n >= 0 && n <= 255`
4. Add `FromRedisValue for f64`:
   - Match `RedisValue::BulkString(b)`, parse bytes as `f64::from_utf8` then `f64::from_str`
   - Handle UTF-8 decode errors and parse errors

## Verification

### Unit Tests

- [ ] `test_from_redis_value_u64_zero` — parses from Integer(0)
- [ ] `test_from_redis_value_u64_max` — parses from Integer(u64::MAX)
- [ ] `test_from_redis_value_u64_overflow` — returns error from Integer(i64::MAX + 1)
- [ ] `test_from_redis_value_i32_zero`, `i32::MAX`, `i32::MIN` — all pass
- [ ] `test_from_redis_value_u8_zero`, `u8::MAX` — all pass
- [ ] `test_from_redis_value_f64_basic` — parses "3.14" from BulkString
- [ ] `test_from_redis_value_f64_negative` — parses "-1.5" from BulkString
- [ ] `test_from_redis_value_f64_scientific` — parses "1.5e10" from BulkString
- [ ] `test_from_redis_value_u64_wrong_type` — returns error from Integer(0)

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests

- [ ] Test INCRBYFLOAT returns `f64` through the new impl
- [ ] Test HGET with integer value returns `i32` through the new impl

### Expected Results

- 4 new `FromRedisValue` impls
- Safe range-checked conversions
- All existing tests still pass
- Clippy clean
