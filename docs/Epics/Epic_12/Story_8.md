# Story 12.8 — Integration tests for FromRedisValue new-type return types

| Field       | Value                                              |
|-------------|----------------------------------------------------|
| **Story**   | 12.8                                               |
| **Epic**    | [Story 0](/home/casibbald/Workspace/microscaler/may_redis/docs/Epics/Epic_12/Story_0.md) (Epic overview) |
| **Dependencies** | Story 5 (usize boundary), Story 6 (i32 boundary), Story 7 (f64 edge cases) |
| **Status**  | New                                                |

## Objective

Add integration-style tests that verify the new `FromRedisValue` impls for `u64`, `i32`, `u8`, and `f64` work correctly when used as the return type for actual Redis commands (e.g., `client.execute::<i32>(client.incrby("key", 5))`).

## Epic

12 — Test Gap Remediation

## Dependencies

- Story 5 (usize boundary unit tests)
- Story 6 (i32 boundary unit tests)
- Story 7 (f64 edge case unit tests)

## Source docs

`docs/code-review-2026-05-28.md` (Finding T2 — INFO from audit)

## Finding

T2 (from audit) — While unit tests cover the `FromRedisValue` impls directly, there are no integration tests that verify these types work correctly when used as the return type for actual Redis commands.

## Background

The audit noted that the existing integration test suite uses `#[ignore = "requires live Redis server"]` and requires a real Redis server. This story asks for integration tests that exercise the full path: RedisValue response (from a real Redis command) → `FromRedisValue` impl → typed result.

The existing test infrastructure already has the pattern for this:
- `run_may()` wrapper in `src/client/client.rs` tests
- `shared_client()` for a single shared connection across integration tests
- All tests are tagged `#[ignore = "requires live Redis server"]`

The `InMemoryClient` (available under the `test` feature) does not support all commands needed for these integration tests (e.g., `STRLEN`, `INCRBYFLOAT`, `SADD`), so the tests will use the live Redis integration pattern. This is intentional — the goal is to verify the full command-response → type conversion path.

## Functional Requirements

- [ ] Test that `execute::<u64>` works with Integer responses (e.g., STRLEN)
- [ ] Test that `execute::<i32>` works with Integer responses (e.g., INCR, EXISTS)
- [ ] Test that `execute::<u8>` works with Integer responses (e.g., STRLEN of short string)
- [ ] Test that `execute::<f64>` works with BulkString responses (e.g., INCRBYFLOAT)
- [ ] Test that overflow cases produce errors when used with execute

## Non-Functional Requirements

- [ ] Tests use `may::run` / `may::go` (never `#[tokio::test]` or `.await`)
- [ ] Tests use the `run_may()` + `shared_client()` pattern from existing integration tests
- [ ] Tests are tagged `#[ignore = "requires live Redis server"]`
- [ ] No `unwrap()` on expected-error assertions

## Code Anchors

- `src/core/from_value.rs:86-141` — new `FromRedisValue` impls for `u64`, `i32`, `u8`, `f64`
- `src/client/client.rs:246-248` — `execute<T>()` method
- `src/client/client.rs:368-1186` — existing integration test patterns

## Implementation Tasks

1. Add test `test_integration_u64_return_type`:
   - SET a key with a multi-character value
   - STRLEN the key, read result as `u64` via `FromRedisValue`
   - Verify the length matches

2. Add test `test_integration_i32_return_type`:
   - INCR a key, read result as `i32`
   - EXISTS a key, read result as `i32` (Redis returns 0 or 1 as integer)
   - Verify the values

3. Add test `test_integration_u8_return_type`:
   - SET a short key (3 chars), STRLEN it, read as `u8`
   - Verify the length fits in u8 range (0..255)
   - Also test u8 overflow case: SET a 256-char value, STRLEN, expect error

4. Add test `test_integration_f64_return_type`:
   - INCRBYFLOAT a key by 2.5, read result as `f64`
   - Verify the result matches the increment value

5. Add test `test_integration_mget_with_various_types`:
   - MGET a set of keys
   - Verify the Array response contains correct typed values
   - Test that `Vec<u64>`, `Vec<i32>`, `Vec<u8>`, `Vec<f64>` can be parsed from MGET array results

## Verification

### Unit Tests

- [ ] All 335+ existing tests still pass
- [ ] `cargo test --lib --all-features` — clean

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests (require Redis)

- [ ] `test_integration_u64_return_type` — STRLEN returns i64, convert to u64 via `FromRedisValue`
- [ ] `test_integration_i32_return_type` — EXISTS returns 0 or 1, read as i32
- [ ] `test_integration_u8_return_type` — STRLEN of a short string, read as u8
- [ ] `test_integration_f64_return_type` — INCRBYFLOAT returns bulk string, read as f64
- [ ] `test_integration_mget_with_various_types` — MGET array, verify element types

### Expected Results

- 5 new integration tests
- All existing tests still pass

### Acceptance Criteria

- [ ] New `FromRedisValue` impls work correctly as return types for `execute<T>()`
- [ ] u64 correctly converts Redis integers without overflow
- [ ] i32 correctly converts Redis integers without overflow
- [ ] u8 correctly converts Redis integers in range 0..255
- [ ] f64 correctly converts Redis BulkString floating-point values
- [ ] Error cases produce descriptive Parse errors
- [ ] All 335+ existing tests still pass
- [ ] No tokio or `.await` anywhere
- [ ] All tests use `run_may()` pattern (not `#[tokio::test]`)
