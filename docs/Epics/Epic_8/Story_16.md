# Story 8.16 — RESPReader: Enforce Bulk String Length Cap

**Objective:** Add a maximum bulk string length cap in `RESPReader` to prevent out-of-memory allocation from a malicious server sending absurd bulk string length headers. The current code parses the length as `isize` and allocates `len` bytes without any cap.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure codec change).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #12-15, MEDIUM — data fidelity, overflow, OOM), `src/codec/reader.rs` lines 131-154

## The Problems

### 16a. Bulk string length parsing

```rust
fn read_bulk(&mut self) -> Result<RedisValue, RedisError> {
    let len = std::str::from_utf8(line)
        .map_err(...)?
        .parse::<isize>()
        .map_err(...)?;
    match len.cmp(&0) {
        Less => Ok(RedisValue::Null),
        Equal => ...,
        Greater => {
            let data = self.read_bytes(len as usize)?;  // potential overflow on 32-bit
            Ok(RedisValue::BulkString(data.to_vec()))
        }
    }
}
```

Problems:
- On 32-bit platforms, `isize::MAX` = `2^31-1`. A bulk string of `2^31` bytes would overflow when cast to `usize`.
- On 64-bit, `isize::MAX` = `2^63-1`. A single bulk string of 8 EB would OOM the process.
- No practical cap is enforced.

### 16b. Array length parsing

```rust
fn read_array(&mut self) -> Result<RedisValue, RedisError> {
    let len = ...parse::<usize>()...;
    let mut items = Vec::with_capacity(len);  // OOM if len is huge
}
```

A server sending `*9223372036854775807\r\n` would try to allocate a Vec of that size.

## Functional Requirements

1. **Bulk string max length:** 256 MB (268,435,456 bytes). This is the default Redis `maxmemory` for single values and covers realistic use cases.
2. **Array max length:** 1 million elements. Redis `mget` with millions of keys is a server-side concern; a client should never need to receive more than 1M elements.
3. `RESPReader::with_max_bulk_len(cap: usize)` — configurable bulk string cap.
4. `RESPReader::with_max_array_len(cap: usize)` — configurable array cap.
5. When exceeded, return `Err(RedisError::Parse(format!("bulk string length {} exceeds maximum of {}", len, max)))`.

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **No performance regression** — cap check is O(1) before allocation.
3. **Configurable** — users who need larger values can configure via builder methods.
4. **Error is descriptive** — must include both the received length and the maximum.

## Code Anchors

- `src/codec/reader.rs` — `RESPReader` struct, `read_bulk()`, `read_array()`

## Tasks

1. Add `max_bulk_len: usize` and `max_array_len: usize` fields to `RESPReader`.
2. Add `RESPReader::with_max_bulk_len()` and `with_max_array_len()` builder methods.
3. Add cap check in `read_bulk()` before allocating.
4. Add cap check in `read_array()` before allocating Vec.
5. Write unit tests for cap enforcement.
6. Verify existing tests still pass (default caps are permissive).

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `bulk_under_cap` | `$1000\r\n{1000 bytes}\r\n` | `Ok(BulkString)` |
| `bulk_over_cap_default` | `$268435457\r\n{data}\r\n` | `Err(Parse("exceeds maximum of 268435456"))` |
| `bulk_custom_cap` | `with_max_bulk_len(100)` → `$101\r\n...` | `Err(Parse("exceeds maximum of 100"))` |
| `bulk_null_ok` | `$-1\r\n` | `Ok(RedisValue::Null)` (null is never capped) |
| `bulk_zero_ok` | `$0\r\n\r\n` | `Ok(BulkString([]))` (empty string is OK) |
| `array_under_cap` | `*1000000\r\n...` | `Ok(Array)` |
| `array_over_cap_default` | `*1000001\r\n...` | `Err(Parse("exceeds maximum of 1000000"))` |
| `array_custom_cap` | `with_max_array_len(10)` → `*11\r\n...` | `Err(Parse("exceeds maximum of 10"))` |
| `array_empty_ok` | `*0\r\n` | `Ok(Array([]))` (empty array is OK) |
| `isize_overflow_32bit` | `$2147483648\r\n...` | `Err(Parse(...))` (cannot even parse as isize) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Default bulk cap: 256 MB
- [ ] Default array cap: 1 million elements
- [ ] Custom caps via builder methods work
- [ ] Null bulk strings are not affected by cap
- [ ] Error messages include both received value and maximum
