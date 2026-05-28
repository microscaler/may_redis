# Story 11.12 — Add upper-bound check for `usize` conversion on 32-bit platforms

**Objective:** Add an upper-bound check when converting `i64` to `usize` in `FromRedisValue for usize` to prevent silent truncation on 32-bit platforms.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding T1, LOW)

**Finding:** T1 — `FromRedisValue for usize` casts `i64` to `usize` with `*n as Self`. On 32-bit platforms, this could silently truncate values > 2^32. The check `*n >= 0` is present but no upper-bound check exists.

## Functional Requirements

- [ ] In `FromRedisValue for usize`, add an upper-bound check: `if *n > usize::MAX as i64`
- [ ] Return `RedisError::Parse` with a descriptive message when the value exceeds `usize::MAX`
- [ ] The lower-bound check (`*n < 0`) must be preserved

## Non-Functional Requirements

- [ ] On 64-bit platforms, this check is a no-op (i64::MAX <= usize::MAX)
- [ ] On 32-bit platforms, values in range (2^32, 2^63] are properly rejected

## Code Anchors

- `src/core/from_value.rs:70` — The `FromRedisValue for usize` implementation

## Tasks

1. Read the current `FromRedisValue for usize` implementation
2. Add upper-bound check: `if *n < 0 || *n > usize::MAX as i64`
3. Return `Err(RedisError::Parse("value out of range for usize"))` on overflow
4. Preserve the existing lower-bound check

## Verification

### Unit Tests

- [ ] `test_from_redis_value_usize_zero` — still passes (zero value)
- [ ] `test_from_redis_value_usize_positive` — still passes (positive value)
- [ ] `test_from_redis_value_usize_negative` — still returns error
- [ ] `test_from_redis_value_usize_max_32bit` — passes with value = 2^32 - 1
- [ ] `test_from_redis_value_usize_overflow_32bit` — returns error with value = 2^32 (on 32-bit build target)
- [ ] `test_from_redis_value_usize_i64_max` — passes on 64-bit, returns error on 32-bit

### Cross-platform Verification

- [ ] Verify the fix compiles on both 32-bit and 64-bit targets
- [ ] On 64-bit: `i64::MAX > usize::MAX` is true, so the check is meaningful

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- `usize` conversion is safe on both 32-bit and 64-bit platforms
- Overflow is detected and returns a descriptive error
- All existing tests pass
- Clippy clean
