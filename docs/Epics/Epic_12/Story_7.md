# Story 12.7 — f64 edge cases (SimpleString, empty, inf/nan, exponents)

| Field       | Value                                              |
|-------------|----------------------------------------------------|
| **Story**   | 12.7                                               |
| **Epic**    | [Story 0](/home/casibbald/Workspace/microscaler/may_redis/docs/Epics/Epic_12/Story_0.md) (Epic overview) |
| **Dependencies** | Story 0                                        |
| **Status**  | New                                                |

## Objective

Add edge-case tests for `FromRedisValue for f64` — Redis-specific wire format (SimpleString), empty strings, infinity, NaN, and various numeric formats.

## Epic

12 — Test Gap Remediation

## Dependencies

Story 0 (Epic overview)

## Source docs

`docs/code-review-2026-05-28.md` (Finding T2 — LOW-MEDIUM from audit)

## Finding

The audit identified that the f64 impl only covers basic cases. Missing edge cases are the most numerous — Redis can return floats in several unexpected formats.

## Background

The current f64 impl handles:
- `BulkString(b"1.23")` -> 1.23 ✓
- `BulkString(b"-1.5")` -> -1.5 ✓
- `BulkString(b"1.5e10")` -> 1.5e10 ✓
- `BulkString(b"0.0")` -> 0.0 ✓
- `Integer(42)` -> error ✓
- `BulkString([0xff, 0xfe])` -> error ✓
- `BulkString(b"not a number")` -> error ✓

Missing edge cases that the audit identified:

### Redis-specific formats
1. **SimpleString("inf")** — Redis INCRBYFLOAT can return infinity as SimpleString, not BulkString. Current impl only handles BulkString.
2. **SimpleString("-inf")** — negative infinity
3. **BulkString(b"inf")** — Redis returns "inf" not "Infinity"
4. **BulkString(b"-inf")** — negative infinity
5. **BulkString(b"nan")** — NaN (Redis returns lowercase)

### Parse edge cases
6. **BulkString(b"")** — empty string
7. **BulkString(b"  ")** — whitespace only
8. **BulkString(b"3.14abc")** — trailing garbage
9. **BulkString(b"+1.5")** — explicit positive sign
10. **BulkString(b"-0.0")** — negative zero
11. **BulkString(b"1e-10")** — scientific with negative exponent
12. **BulkString(b"1_000.5")** — underscore separators (Rust `f64::from_str` does NOT support underscores in floats — should error)
13. **BulkString(b"1.2.3")** — multiple decimal points
14. **BulkString(b".")** — decimal point only

### Magnitude edge cases
15. **BulkString(b"1e308")** — near DBL_MAX
16. **BulkString(b"1e-324")** — near DBL_MIN

## Functional Requirements

- [ ] Test Redis infinity formats (inf, -inf)
- [ ] Test Redis NaN format
- [ ] Test empty string handling
- [ ] Test negative zero
- [ ] Test scientific notation with negative exponent
- [ ] Test underscore separators
- [ ] Test near-DBL-MAX and near-DBL-MIN magnitudes

## Non-Functional Requirements

- [ ] Tests use `#[test]` (no may runtime needed)
- [ ] Tests must not use `unwrap()` on expected-error cases
- [ ] Tests for floating-point equality must use epsilon comparison

## Code Anchors

- `src/core/from_value.rs:127-141` — `FromRedisValue for f64` implementation
- `src/core/from_value.rs:353-400` — existing f64 tests

## Implementation Tasks

Add the following tests:

### Redis infinity/NaN (happy path — these parse successfully)
1. `test_from_redis_value_f64_inf` — BulkString(b"inf") → inf
2. `test_from_redis_value_f64_neg_inf` — BulkString(b"-inf") → -inf
3. `test_from_redis_value_f64_nan` — BulkString(b"nan") → NaN (use `f64::is_nan()`)

### Empty/garbage (error path)
4. `test_from_redis_value_f64_empty` — BulkString(b"") → error
5. `test_from_redis_value_f64_whitespace` — BulkString(b"  ") → error
6. `test_from_redis_value_f64_trailing_garbage` — BulkString(b"3.14abc") → error
7. `test_from_redis_value_f64_multiple_decimals` — BulkString(b"1.2.3") → error
8. `test_from_redis_value_f64_decimal_only` — BulkString(b".") → error

### Edge formats (happy + error path)
9. `test_from_redis_value_f64_neg_zero` — BulkString(b"-0.0") → -0.0
10. `test_from_redis_value_f64_exp_neg` — BulkString(b"1e-10") → 1e-10
11. `test_from_redis_value_f64_underscores` — BulkString(b"1_000.5") → error (Rust `f64::from_str` does NOT support underscores in float strings)
12. `test_from_redis_value_f64_exp_sign` — BulkString(b"+1.5") → 1.5

### Magnitude edge cases
13. `test_from_redis_value_f64_near_max` — BulkString(b"1e308") → large finite
14. `test_from_redis_value_f64_near_min` — BulkString(b"1e-324") → small positive

## Verification

### Unit Tests

- [ ] 14 new unit tests covering Redis-specific and Rust parse edge cases
- [ ] Tests verify both happy path and error path

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- 14 new unit tests
- All existing tests still pass

### Acceptance Criteria

- [ ] Redis infinity values ("inf", "-inf") parse to f64::INFINITY and f64::NEG_INFINITY
- [ ] Redis NaN ("nan") parses to f64::NAN
- [ ] Empty string returns Parse error
- [ ] Negative zero (-0.0) parses to negative zero
- [ ] Scientific notation with negative exponent works
- [ ] Underscore number separators parse correctly
- [ ] Near-DBL-MAX and near-DBL-MIN values parse correctly
- [ ] Garbage/trailing characters return Parse error
- [ ] No unwrap() on expected-error cases
- [ ] All f64 tests use epsilon comparison for equality checks
