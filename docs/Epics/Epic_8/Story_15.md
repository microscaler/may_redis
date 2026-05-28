# Story 8.15 — RESPReader: Enforce CRLF After Every Value

**Objective:** Make the RESPReader strictly enforce CRLF line terminators after every parsed value. Currently, `skip_crlf()` is optional — if the server sends values without CRLF between them, the parser silently misinterprets the next value's marker as data. For RESP2, CRLF after every value is mandatory.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure codec change).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #11, MEDIUM), `src/codec/reader.rs`

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

For example, if the server sends `+OK\rnPONG\r\n` (missing CRLF after OK), the `P` of PONG becomes the first byte of the next value. The parser would try to read a type marker from `P`, get `RedisError::Parse("unknown RESP marker: 80")` (ASCII 80 = 'P').

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

## Implementation

### Changes to `src/codec/reader.rs`

1. **Added `expect_crlf()`** — returns `Result<(), RedisError>`, enforces mandatory `\r\n` after every value. If buffer is exhausted, returns `Ok(())` (nothing more to read).

2. **Replaced `read_line()`** — now returns `Vec<u8>` (owned) instead of `&[u8]` (borrow) to avoid borrow conflicts with `expect_crlf()`. Stops at `\r` without consuming it, returning the line content before it. Rejects bare `\n` (not preceded by `\r`) as invalid.

3. **Replaced `read_bytes()`** — now returns `Vec<u8>` (owned) instead of `&[u8]` (borrow) for the same reason.

4. **Updated all value readers** (`read_simple`, `read_error`, `read_integer`, `read_bulk`, `read_array`) — each now calls `expect_crlf()` after parsing its content to enforce CRLF termination.

5. **Kept `skip_crlf()`** — pre-existing silent CRLF skip before reading the marker on each `read_value()` call (handles inter-value CRLF between previously-read values).

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `crlf_ok_simple` | `+OK\r\n` | `Ok(SimpleString("OK"))` |
| `crlf_ok_int` | `:42\r\n` | `Ok(Integer(42))` |
| `crlf_ok_bulk` | `$5\r\nhello\r\n` | `Ok(BulkString("hello"))` |
| `crlf_ok_array` | `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` | `Ok(Array(...))` |
| `crlf_missing_after_simple` | `+OK\rPONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_missing_after_int` | `:42\rPONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_missing_after_bulk` | `$5\r\nhello\rPONG\r\n` | `Err(Parse("expected CRLF"))` |
| `crlf_double_lf` | `+OK\n\r\n` | `Err(Parse("expected CRLF"))` (LF before CR) |

## Verification Checklist

- [x] `cargo check --lib` passes
- [x] `cargo test --lib` — all 284 tests pass (21 new CRLF tests added, 263 existing tests pass)
- [x] `cargo clippy --lib` — zero new warnings from this story (pre-existing clippy errors in `pipeline.rs` and `client.rs` are unrelated)
- [x] Well-formed RESP2 with CRLF still parses correctly
- [x] Missing CRLF produces a clear parse error
- [x] Double CRLF (`\r\n\r\n`) is accepted (extra CRLF between values is allowed via `skip_crlf()`)
