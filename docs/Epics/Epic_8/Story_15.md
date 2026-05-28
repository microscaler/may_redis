# Story 8.15 — RESPReader: Enforce CRLF After Every Value

**Objective:** Make the RESPReader strictly enforce CRLF line terminators after every parsed value. Currently, `skip_crlf()` is optional — if the server sends values without CRLF between them, the parser silently misinterprets the next value's marker as data. For RESP2, CRLF after every value is mandatory.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure codec change).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #11, MEDIUM), `src/codec/reader.rs` lines 34-65

## The Problem

```rust
fn skip_crlf(&mut self) {
    if self.pos + 2 <= self.buf.len()
        && self.buf[self.pos] == b'\r'
        && self.buf[self.pos + 1] == b'\n'
    {
        self.pos += 2;
    }
    // If no CRLF, silently continues without error
}
```

When `read_value()` is called after a value:
- If CRLF is present → advances 2 bytes, next value parsed correctly
- If CRLF is NOT present → does nothing, and the next value's first byte is treated as data

For example, if the server sends `+OK\r\nPONG\r\n` (missing CRLF after OK), the `P` of PONG becomes the first byte of the next value. The parser would try to read a type marker from `P`, get `RedisError::Parse("unknown RESP marker: 80")` (ASCII 80 = 'P').

**Impact:** A buggy server sending values without CRLF would cause parse errors that are hard to diagnose. With strict enforcement, the error would be: `Parse("expected CRLF after value")`.

## Functional Requirements

1. After each successfully parsed value, the reader must check for CRLF.
2. If CRLF is missing, return `Err(RedisError::Parse("expected CRLF after value"))`.
3. `read_value()` must call the CRLF check after parsing completes, not before.
4. Empty buffers (no data) should still return an error, not silently succeed.

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **Existing well-formed data must parse** — all existing tests use proper CRLF, so they must continue to pass.
3. **Error message** — must clearly state "expected CRLF" so users understand the issue.
4. **Performance** — CRLF check is O(1), no performance impact.

## Code Anchors

- `src/codec/reader.rs` — `read_value()`, `skip_crlf()`, `read_line()`, `read_bytes()`

## Tasks

1. Rename `skip_crlf()` to `expect_crlf()` and make it return `Result`.
2. Call `expect_crlf()` after each successful value parse in `read_value()`.
3. Also call `expect_crlf()` after `read_array()` and `read_bulk()` (they already handle CRLF internally for their content, but not for the trailing CRLF after the value).
4. Write test for missing CRLF → error.
5. Write test for well-formed CRLF → success (regression guard).

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `crlf_ok_simple` | `+OK\r\n` | `Ok(SimpleString("OK"))` |
| `crlf_ok_int` | `:42\r\n` | `Ok(Integer(42))` |
| `crlf_ok_bulk` | `$5\r\nhello\r\n` | `Ok(BulkString("hello"))` |
| `crlf_ok_array` | `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` | `Ok(Array(...))` |
| `crlf_missing_after_simple` | `+OKPONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_missing_after_int` | `:42PONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_missing_after_bulk` | `$5\r\nhelloPONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_double_lf` | `+OK\n\r\n` | `Err(Parse("expected CRLF"))` (LF before CR) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Well-formed RESP2 with CRLF still parses correctly
- [ ] Missing CRLF produces a clear parse error
- [ ] Double CRLF (`\r\n\r\n`) is accepted (extra CRLF between values is allowed)
