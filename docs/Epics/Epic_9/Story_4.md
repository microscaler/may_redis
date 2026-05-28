# Story 9.4 — Roundtrip Invariant Tests for All Value Types

**Objective:** Add comprehensive invariant tests that verify the roundtrip integrity of every `RedisValue` variant through the RESP codec. These tests ensure `write_value → read_value` is an identity function for all possible input values.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** None (pure test addition).

**Source docs:**
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — "JSF AV Rules Compliance" and performance validation
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — "Stack-allocated collections" validation
- `src/codec/roundtrip.rs` — existing roundtrip tests

## The Gap

The existing `roundtrip.rs` tests cover common cases but may miss edge cases. Under JSF AV compliance, we need to prove that the codec is correct for ALL value types, not just the happy path.

## Functional Requirements

1. Add roundtrip tests for ALL `RedisValue` variants with edge case data.
2. Each test: write → read → compare equality.
3. Binary bulk strings must survive roundtrip without UTF-8 conversion.
4. Mixed-type arrays must preserve type information through roundtrip.
5. Error values with `ERR:` prefix must roundtrip correctly.

## Non-Functional Requirements

1. **No new dependencies.**
2. **Zero may dependency** — codec has no may imports.
3. **No code changes** — this story is test-only.
4. **Deterministic** — all tests must be deterministic.

## Code Anchors

- `src/codec/roundtrip.rs` — existing roundtrip tests
- `src/codec/writer.rs` — `RESPWriter`
- `src/codec/reader.rs` — `RESPReader`

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `roundtrip_simple_string` | `SimpleString("OK")` | Write → Read → `SimpleString("OK")` |
| `roundtrip_simple_space` | `SimpleString("OK with spaces")` | Write → Read → identical |
| `roundtrip_error_prefix` | `Error("ERR wrong type")` | Write → Read → `Error("ERR wrong type")` |
| `roundtrip_error_auth` | `Error("-NOAUTH Authentication required.")` | Write → Read → identical |
| `roundtrip_integer_zero` | `Integer(0)` | Write → Read → `Integer(0)` |
| `roundtrip_integer_positive` | `Integer(42)` | Write → Read → `Integer(42)` |
| `roundtrip_integer_negative` | `Integer(-1)` | Write → Read → `Integer(-1)` |
| `roundtrip_integer_max` | `Integer(i64::MAX)` | Write → Read → `Integer(i64::MAX)` |
| `roundtrip_integer_min` | `Integer(i64::MIN)` | Write → Read → `Integer(i64::MIN)` |
| `roundtrip_bulk_empty` | `BulkString([])` | Write → Read → `BulkString([])` |
| `roundtrip_bulk_ascii` | `BulkString(b"hello")` | Write → Read → `BulkString(b"hello")` |
| `roundtrip_binary_non_utf8` | `BulkString([0x00, 0xFF, 0x80, 0x7F])` | Write → Read → identical bytes |
| `roundtrip_array_empty` | `Array([])` | Write → Read → `Array([])` |
| `roundtrip_array_single` | `Array([Integer(1)])` | Write → Read → identical |
| `roundtrip_mixed_array` | `Array([Integer(1), BulkString(b"hi"), Null])` | Write → Read → identical types |
| `roundtrip_nested_array` | `Array([Array([Integer(42)])])` | Write → Read → identical |
| `roundtrip_nested_error_in_array` | `Array([Error("ERR x")])` | Write → Read → `Array([Error("ERR x")])` |
| `roundtrip_null` | `Null` | Write → Read → `Null` |
| `roundtrip_deep_nesting` | `Array([Array([Array([Integer(42)])])])` (5 levels) | Write → Read → identical |
| `roundtrip_many_elements` | `Array([Integer(i); 1000])` | Write → Read → 1000 identical integers |
| `roundtrip_large_bulk` | `BulkString(vec![b'a'; 65536])` | Write → Read → identical 64KB bulk string |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 300+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Every `RedisValue` variant roundtrips correctly
- [ ] Binary data (non-UTF-8) survives roundtrip without corruption
- [ ] Deep nesting (5+ levels) survives roundtrip
- [ ] Large values (64KB bulk, 1000-element array) survive roundtrip
