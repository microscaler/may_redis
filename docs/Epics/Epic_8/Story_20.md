# Story 8.20 — RESP Codec: Add Invariant Tests for All Value Types

**Objective:** Add comprehensive invariant tests that verify the roundtrip integrity of every `RedisValue` variant through the RESP codec. These tests ensure that `write_value → read_value` is an identity function for all possible input values, catching any encoder/decoder mismatches.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure test addition).

**Source docs:** `docs/redis-implementation-audit.md` (general coverage gap), `src/codec/roundtrip.rs`

## The Gap

The existing `roundtrip.rs` tests cover common cases but miss some edge cases and value variants. Specifically:
- `SimpleString` with special characters (spaces, equals signs, etc.)
- `Error` with colon-separated prefixes (Redis error format: `-ERR msg`)
- `Array` containing mixed types
- `BulkString` with binary data (non-UTF-8 bytes)
- Nested arrays with errors inside
- Empty array, empty bulk string, null
- Integer edge cases in roundtrip

## Functional Requirements

1. Add roundtrip tests for ALL `RedisValue` variants with edge case data.
2. Each test: write → read → compare equality.
3. Binary bulk strings must survive roundtrip without UTF-8 conversion.
4. Mixed-type arrays must preserve type information through roundtrip.
5. Error values with `ERR:` prefix must roundtrip correctly.

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **No code changes** — this story is test-only.
3. **Deterministic** — all tests must be deterministic (no randomness).

## Code Anchors

- `src/codec/roundtrip.rs` — existing roundtrip tests
- `src/codec/writer.rs` — `RESPWriter`
- `src/codec/reader.rs` — `RESPReader`

## Tasks

1. Add roundtrip tests for all `RedisValue` edge cases.
2. Run `cargo test` to confirm all pass.
3. Verify binary data roundtrips correctly.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `roundtrip_simple_space` | `SimpleString("OK with spaces")` | Write → Read → `SimpleString("OK with spaces")` |
| `roundtrip_error_prefix` | `Error("ERR wrong type")` | Write → Read → `Error("ERR wrong type")` |
| `roundtrip_binary_bulk` | `BulkString([0x00, 0xFF, 0x80, 0x7F])` | Write → Read → `BulkString([0x00, 0xFF, 0x80, 0x7F])` |
| `roundtrip_mixed_array` | `Array([Integer(1), BulkString(b"hi"), Null])` | Write → Read → identical |
| `roundtrip_nested_error` | `Array([Error("ERR x")])` | Write → Read → `Array([Error("ERR x")])` |
| `roundtrip_empty_array` | `Array([])` | Write → Read → `Array([])` |
| `roundtrip_empty_bulk` | `BulkString([])` | Write → Read → `BulkString([])` |
| `roundtrip_null` | `Null` | Write → Read → `Null` |
| `roundtrip_deeply_nested` | `Array([Array([Array([Integer(42)])])])` | Write → Read → identical |
| `roundtrip_all_ints` | `Array([Integer(-1), Integer(0), Integer(1), Integer(i64::MAX), Integer(i64::MIN)])` | Write → Read → identical |
| `roundtrip_large_bulk` | `BulkString(vec![b'a'; 65536])` | Write → Read → identical |
| `roundtrip_multiple_values` | Multiple values back-to-back | Each value roundtrips independently |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Every `RedisValue` variant roundtrips correctly
- [ ] Binary data (non-UTF-8) survives roundtrip without corruption
- [ ] Deep nesting survives roundtrip
- [ ] Multiple values back-to-back parse correctly
