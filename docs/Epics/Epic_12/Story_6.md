# Story 12.6 — i32 boundary tests (i32::MAX+1, i32::MIN-1)

| Field       | Value                                              |
|-------------|----------------------------------------------------|
| **Story**   | 12.6                                               |
| **Epic**    | [Story 0](/home/casibbald/Workspace/microscaler/may_redis/docs/Epics/Epic_12/Story_0.md) (Epic overview) |
| **Dependencies** | Story 0                                        |
| **Status**  | New                                                |

## Objective

Add boundary tests for `FromRedisValue for i32` covering values just outside the i32 range (i32::MAX + 1 and i32::MIN - 1) and values just inside the range (i32::MAX - 1 and i32::MIN + 1).

## Epic

12 — Test Gap Remediation

## Dependencies

Story 0 (Epic overview)

## Source docs

`docs/code-review-2026-05-28.md` (Finding T2 — LOW-MEDIUM from audit)

## Finding

The current test for `FromRedisValue for i32` only tests `i64::MAX` as the overflow case. It does NOT test the values that are just outside i32 range: `i32::MAX + 1` (= 2147483648) and `i32::MIN - 1` (= -2147483649). These are the first values that should fail.

## Background

The audit found that the i32 impl only tests:
- 0 (success)
- i32::MAX (success)
- i32::MIN (success)
- i64::MAX (failure — far outside range)

What's missing are the **boundary** cases — the exact values where the round-trip cast check fails:
- i32::MAX as i32 as i64 = i32::MAX ✓ (should pass)
- (i32::MAX + 1) as i32 as i64 = i32::MIN ≠ i32::MAX+1 ✗ (should fail)
- i32::MIN as i32 as i64 = i32::MIN ✓ (should pass)
- (i32::MIN - 1) as i32 as i64 = i32::MAX ≠ i32::MIN-1 ✗ (should fail)

The round-trip check `i64::from(*n as Self) == *n` works by casting the i64 to i32 (which wraps on overflow) then back to i64 and comparing. If the values differ, the original i64 was out of range. This is correct, but the first-failure values are untested.

## Functional Requirements

- [ ] Test that `i32::MAX + 1` returns Parse error
- [ ] Test that `i32::MIN - 1` returns Parse error
- [ ] Test that `i32::MAX - 1` (just inside range) succeeds
- [ ] Test that `i32::MIN + 1` (just inside range) succeeds

## Non-Functional Requirements

- [ ] Tests use `#[test]` (no may runtime needed — pure data conversion)
- [ ] Tests must not use `unwrap()` on expected-error cases
- [ ] Tests must use `i64::from(i32::MAX) + 1` and similar to avoid overflow in test setup

## Code Anchors

- `src/core/from_value.rs:100-111` — `FromRedisValue for i32` implementation
- `src/core/from_value.rs:305-322` — existing i32 tests

## Implementation Tasks

1. Add test `test_from_redis_value_i32_overflow_positive`:
   ```rust
   #[test]
   fn test_from_redis_value_i32_overflow_positive() {
       let val = RedisValue::Integer(i64::from(i32::MAX) + 1);
       let result: Result<i32, _> = FromRedisValue::from_redis_value(&val);
       assert!(result.is_err());
   }
   ```

2. Add test `test_from_redis_value_i32_overflow_negative`:
   ```rust
   #[test]
   fn test_from_redis_value_i32_overflow_negative() {
       let val = RedisValue::Integer(i64::from(i32::MIN) - 1);
       let result: Result<i32, _> = FromRedisValue::from_redis_value(&val);
       assert!(result.is_err());
   }
   ```

3. Add test `test_from_redis_value_i32_near_max`:
   ```rust
   #[test]
   fn test_from_redis_value_i32_near_max() {
       let val = RedisValue::Integer(i64::from(i32::MAX) - 1);
       let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
       assert_eq!(result, i32::MAX - 1);
   }
   ```

4. Add test `test_from_redis_value_i32_near_min`:
   ```rust
   #[test]
   fn test_from_redis_value_i32_near_min() {
       let val = RedisValue::Integer(i64::from(i32::MIN) + 1);
       let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
       assert_eq!(result, i32::MIN + 1);
   }
   ```

## Verification

### Unit Tests

- [ ] 4 new unit tests covering i32 boundary values
- [ ] Tests verify both inside-range and outside-range at the boundary

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- 4 new unit tests
- All existing tests still pass

### Acceptance Criteria

- [ ] i32::MAX + 1 (2147483648) returns Parse error
- [ ] i32::MIN - 1 (-2147483649) returns Parse error
- [ ] i32::MAX - 1 (2147483646) parses successfully
- [ ] i32::MIN + 1 (-2147483647) parses successfully
- [ ] No unwrap() on expected-error cases
