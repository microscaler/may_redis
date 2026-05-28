# Story 8.17 — RESPError Parsing: Preserve Binary Error Messages

**Objective:** Fix `RESPReader::read_error()` to return `RedisValue::Error` with the raw bytes when the error message is not valid UTF-8. Currently, `String::from_utf8_lossy` silently replaces invalid bytes with the Unicode replacement character (U+FFFD), corrupting the error message.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure codec change).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #13, MEDIUM — data fidelity), `src/codec/reader.rs` lines 110-115

## The Problem

```rust
fn read_error(&mut self) -> Result<RedisValue, RedisError> {
    let line = self.read_line()?;
    Ok(RedisValue::Error(
        String::from_utf8_lossy(line).into_owned(),  // corrupts non-UTF-8
    ))
}
```

If a Redis error message contains non-UTF-8 bytes (unlikely but possible through binary protocol abuse or a misconfigured Redis module), `from_utf8_lossy` replaces them with `�` (U+FFFD). The original error message is corrupted.

**Impact:** A malicious or buggy server could send error messages with embedded non-UTF-8 bytes. While unlikely in practice (Redis error messages are typically ASCII), preserving the raw bytes is the correct behavior for a protocol library.

## Functional Requirements

1. `RESPReader::read_error()` must return `RedisValue::Error(Bytes)` (raw bytes) instead of `RedisValue::Error(String)`.
2. **OR** — simpler approach: keep `RedisValue::Error(String)` but use a lossy fallback only when the application layer needs a string representation.
3. **Recommended:** Keep the current `RedisValue::Error(String)` API but add a separate `read_error_bytes()` method that returns `Vec<u8>`, or change `RedisValue` to have `Error(Vec<u8>)`.

Since changing `RedisValue::Error` from `String` to `Vec<u8>` would be a breaking change across the entire codebase, the safer approach is:
1. Add `read_error_bytes(&mut self) -> Result<Vec<u8>, RedisError>` method.
2. Keep `read_error()` as-is for backwards compatibility.
3. In `connection/connection.rs`, use `read_error_bytes()` and convert with `from_utf8_lossy` only for logging/debugging.

Wait — actually, the cleaner approach for a Redis library is to accept that error messages ARE text and use `from_utf8_lossy` as a reasonable default, but add a test covering the behavior. Let me reconsider.

**Revised approach:** Since Redis error messages are defined as text in the RESP protocol, using `from_utf8_lossy` is acceptable. However, the library should log a warning when non-UTF-8 bytes are detected. The real issue is that `RedisValue::Error(String)` silently loses information. The fix is to:
1. Keep `RedisValue::Error(String)` with lossy conversion for the public API.
2. Add a `RedisValue::RawError(Vec<u8>)` variant for binary error preservation.
3. Update connection layer to prefer `RawError` when parsing error responses.

This is a larger change. For Story 8.17 scope, the simpler fix is:
1. Add a `warn!` log when `from_utf8_lossy` replaces bytes.
2. Document the limitation.
3. Consider `RawError` variant for a future story.

## Functional Requirements

1. `RESPReader::read_error()` must detect non-UTF-8 bytes and log a warning via `log::warn!`.
2. Continue to use `from_utf8_lossy` as the conversion mechanism (backwards compatible).
3. Add `read_error_bytes()` method that returns `Result<Vec<u8>, RedisError>` for applications that need raw bytes.
4. Document the UTF-8 lossy conversion behavior in public API docs.

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **Logging must not panic** — if the log crate is not available, silently ignore the warning.
3. **No performance regression** — UTF-8 validation is O(n) whether or not we warn.

## Code Anchors

- `src/codec/reader.rs` — `read_error()`, `read_line()`

## Tasks

1. In `read_error()`, check if the line is valid UTF-8 before calling `from_utf8_lossy`.
2. If invalid UTF-8, log a warning and continue with lossy conversion.
3. Add `read_error_bytes()` method.
4. Write unit tests for valid and invalid UTF-8 error messages.
5. Document the UTF-8 behavior in `RedisValue::Error` docs.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `error_valid_utf8` | `-ERR normal error\r\n` | `Ok(Error("normal error"))` |
| `error_invalid_utf8` | `-ERR \xff\xfe\r\n` | `Ok(Error("��"))` with log warning |
| `error_bytes_valid` | `read_error_bytes()` on valid UTF-8 | `Ok(vec![...])` |
| `error_bytes_invalid` | `read_error_bytes()` on invalid UTF-8 | `Ok(raw bytes)` (no loss) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Valid UTF-8 error messages parse correctly
- [ ] Invalid UTF-8 error messages produce lossy string + log warning
- [ ] `read_error_bytes()` returns raw bytes regardless of validity
