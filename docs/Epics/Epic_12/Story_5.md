# Story 12.5 — usize exact boundary tests (usize::MAX)

| Field       | Value                                              |
|-------------|----------------------------------------------------|
| **Story**   | 12.5                                               |
| **Epic**    | [Story 0](/home/casibbald/Workspace/microscaler/may_redis/docs/Epics/Epic_12/Story_0.md) (Epic overview) |
| **Dependencies** | Story 0                                        |
| **Status**  | New                                                |

## Objective

Add exact boundary tests for `FromRedisValue for usize` covering `usize::MAX` and one-above-usize::MAX.

## Epic

12 — Test Gap Remediation

## Dependencies

Story 0 (Epic overview)

## Source docs

`docs/code-review-2026-05-28.md` (Finding T1 — LOW)

## Finding

T1 — The `FromRedisValue for usize` uses `u64::try_into(n)` which correctly rejects overflow on 32-bit. But no test exercises the exact `usize::MAX` boundary value or the value just above it.

## Background

The current test suite has `test_from_redis_value_usize_i64_max` which tests i64::MAX (platform-dependent behavior). But two critical boundary values are untested:
1. `usize::MAX` exactly — should succeed
2. `usize::MAX + 1` — should fail

These are the exact values where the `try_into` boundary check activates.

## Functional Requirements

- [ ] Test that `usize::MAX` as an Integer parses successfully
- [ ] Test that `usize::MAX + 1` as an Integer returns Parse error

## Non-Functional Requirements

- [ ] Tests use `#[test]` (no may runtime needed — pure data conversion)
- [ ] Tests must work on both 32-bit and 64-bit platforms
- [ ] Tests must not use `unwrap()` on the expected-error case

## Code Anchors

- `src/core/from_value.rs:68-84` — `FromRedisValue for usize` implementation

## Implementation Tasks

1. Add test `test_from_redis_value_usize_exact_max`:
   ```rust
   #[test]
   fn test_from_redis_value_usize_exact_max() {
       let val = RedisValue::Integer(usize::MAX as i64);
       let result: usize = FromRedisValue::from_redis_value(&val).unwrap();
       assert_eq!(result, usize::MAX);
   }
   ```
   Note: On 32-bit, `usize::MAX as i64` = `0xFFFFFFFF` (4294967295), which fits in i64. On 64-bit, same value. So this test is safe on both platforms.

2. Add test `test_from_redis_value_usize_overflow_by_one`:
   ```rust
   #[test]
   fn test_from_redis_value_usize_overflow_by_one() {
       let val = RedisValue::Integer((usize::MAX as i64) + 1);
       let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
       assert!(result.is_err());
   }
   ```
   On 64-bit: usize::MAX = 18446744073709551615, i64::MAX = 9223372036854775807. So `usize::MAX as i64` would already overflow in Rust. This test needs special handling:
   - On 64-bit: usize::MAX > i64::MAX, so we can't even create `RedisValue::Integer(usize::MAX as i64)` because usize::MAX doesn't fit in i64. The `try_into` in the impl handles the case where i64 is within range but usize is not.
   - On 32-bit: usize::MAX = 4294967295, so `usize::MAX as i64` = 4294967295, and `(usize::MAX as i64) + 1` = 4294967296, which is larger than usize::MAX. This is the boundary to test.

   So the test must be platform-aware:
   ```rust
   #[test]
   fn test_from_redis_value_usize_overflow_boundary() {
       if cfg!(target_pointer_width = "32") {
           let val = RedisValue::Integer((usize::MAX as i64) + 1);
           let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
           assert!(result.is_err());
       }
       // On 64-bit, usize::MAX > i64::MAX, so we can't create a RedisValue::Integer
       // that exceeds usize::MAX. The boundary is already covered by i64::MAX test.
   }
   ```

## Verification

### Unit Tests

- [ ] `test_from_redis_value_usize_exact_max` — usize::MAX parses successfully
- [ ] `test_from_redis_value_usize_overflow_boundary` — usize::MAX + 1 returns error (32-bit only)

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- 2 new unit tests
- All existing tests still pass

### Acceptance Criteria

- [ ] Exact `usize::MAX` value converts without error
- [ ] Value one above `usize::MAX` returns Parse error
- [ ] Tests are portable across 32-bit and 64-bit platforms
- [ ] No unwrap() on expected-error cases
