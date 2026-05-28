# Story 8.19 — RESP Writer: Correct Integer Encoding for Edge Cases

**Objective:** Add comprehensive tests and edge case handling for integer encoding in `RESPWriter::write_int()`. Specifically test negative integers, `i64::MIN`, `i64::MAX`, and the zero edge case. The current implementation uses `itoa::Buffer` which handles all cases correctly, but there are no tests for edge cases.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure test addition).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #14, MEDIUM — parsing edge case), `src/codec/writer.rs` lines 47-52

## The Gap

```rust
pub fn write_int(&mut self, n: i64) {
    self.buf.extend_from_slice(b":");
    self.buf.extend_from_slice(itoa::Buffer::new().format(n).as_bytes());
    self.buf.extend_from_slice(b"\r\n");
}
```

`itoa::Buffer::format(n)` correctly handles all `i64` values, but the test suite only tests `42` and `-1`. Missing edge cases:
- `i64::MIN` = `-9223372036854775808` (19 chars + sign = 20 bytes)
- `i64::MAX` = `9223372036854775807` (19 chars = 19 bytes)
- `0` → `:0\r\n`
- Large negative: `-999999999999`

## Functional Requirements

1. Add unit tests for `i64::MIN`, `i64::MAX`, `0`, and large values.
2. Verify the exact wire format for each edge case.
3. Verify the total byte count matches expectations.
4. No code changes needed — `itoa` handles all cases correctly.

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **No code changes** — this story is test-only.

## Code Anchors

- `src/codec/writer.rs` — `write_int()`

## Tasks

1. Add unit tests for integer edge cases in `writer.rs` tests.
2. Run `cargo test` to confirm all pass.

## Unit Test Plan

| Test Name | Input | Expected Wire | Byte Count |
|-----------|-------|---------------|------------|
| `int_min` | `write_int(i64::MIN)` | `:-9223372036854775808\r\n` | 22 |
| `int_max` | `write_int(i64::MAX)` | `:9223372036854775807\r\n` | 21 |
| `int_zero` | `write_int(0)` | `:0\r\n` | 3 |
| `int_large_neg` | `write_int(-999999999999)` | `:-999999999999\r\n` | 16 |
| `int_large_pos` | `write_int(999999999999)` | `:999999999999\r\n` | 15 |
| `int_one` | `write_int(1)` | `:1\r\n` | 3 |
| `int_neg_one` | `write_int(-1)` | `:-1\r\n` | 4 |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] All edge case wire formats are byte-exact
- [ ] Total byte counts match expectations
