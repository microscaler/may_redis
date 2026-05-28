# Story 8.2 — ToRedisArgs for Remaining Types

**Objective:** Implement `ToRedisArgs` for `bool` and `Vec<&str>` to complement the existing impls (`String`, `&str`, `i64`, `u32`, `f64`, `&[u8]`). This enables users to pass boolean flags and slices of string references directly as command arguments.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1 (FromRedisValue for basic types).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #2, medium), `docs/01-protocol-analysis.md`

## Functional Requirements

1. `bool` converts to `"0"` or `"1"` bytes for RESP wire format. Redis accepts `0`/`1` as boolean responses.
2. `Vec<&str>` serializes each element via `write_redis_args()` using the existing `&str` impl, producing multiple bulk strings in order.
3. `to_redis_args()` for `bool` returns exactly one value: `"0"` or `"1"`.
4. `to_redis_args()` for `Vec<&str>` returns one value per string element, preserving order.

## Non-Functional Requirements

1. **Zero may dependency** — `to_args.rs` has no `may` imports.
2. **Consistent wire format** — `bool` must produce `"0"`/`"1"` (not `"false"`/`"true"`), matching Redis wire conventions.
3. **Backwards compatible** — Existing `ToRedisArgs` impls are unchanged.
4. **No performance regression** — `Vec<&str>` uses `write_redis_args` (not `collect` into intermediate Vec).

## Code Anchors

- `src/core/to_args.rs` — `impl ToRedisArgs for bool`, `impl ToRedisArgs for Vec<&str>`

## Tasks

1. Implement `ToRedisArgs for bool` — converts true→"1", false→"0"
2. Implement `ToRedisArgs for Vec<&str>` — delegates to `&str` impl for each element
3. Write unit tests for each impl

## Unit Test Plan

### bool tests (4 tests):

| Test Name | Input | Expected Wire |
|-----------|-------|---------------|
| `bool_to_args_true` | true → `"$1\r\n1\r\n"` |
| `bool_to_args_false` | false → `"$1\r\n0\r\n"` |
| `bool_in_command` | `set("key", true)` → `*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$1\r\n1\r\n` |

### Vec<&str> tests (4 tests):

| Test Name | Input | Expected Wire |
|-----------|-------|---------------|
| `vec_str_single` | `["hello"]` → `"$5\r\nhello\r\n"` |
| `vec_str_multi` | `["a","b","c"]` → `"$1\r\na\r\n$1\r\nb\r\n$1\r\nc\r\n"` |
| `vec_str_in_command` | `mget(&["k1","k2"])` → `*3\r\n$4\r\nMGET\r\n$2\r\nk1\r\n$2\r\nk2\r\n` |
| `vec_str_empty` | `Vec::<&str>::new()` → no args appended |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 262 tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `bool` wire format is `"0"`/`"1"` (not `"false"`/`"true"`)
- [ ] `Vec<&str>` preserves element order
- [ ] No double-free or borrow issues in `to_redis_args()`
