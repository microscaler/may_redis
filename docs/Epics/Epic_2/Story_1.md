# Story 2.1 — RESPWriter (encode direction)

**Objective:** Implement the RESPWriter for encoding commands into RESP wire format.

**Epic:** 2 — Codec Crate

**Dependencies:** Story 2.0 (epic overview)

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/05-protocol-layer-design.md`

## Code Anchors

- `src/codec/mod.rs` — `pub struct RESPWriter { buf: BytesMut }`
- `src/codec/writer.rs` — implementation

## Struct

```rust
pub struct RESPWriter {
    buf: BytesMut,
}
```

## Methods

```rust
impl RESPWriter {
    pub fn new() -> Self;
    pub fn with_capacity(cap: usize) -> Self;
    pub fn write_simple(&mut self, s: &str);
    pub fn write_bulk(&mut self, data: &[u8]);
    pub fn write_int(&mut self, n: i64);
    pub fn write_array_header(&mut self, len: usize);
    pub fn write_array_value(&mut self, v: &RedisValue);
    pub fn write_null_bulk(&mut self);
    pub fn write_empty_array(&mut self);
    pub fn write_error(&mut self, msg: &str);
    pub fn take(&mut self) -> BytesMut;
}
```

## Tasks

- [x] Define `RESPWriter` with `buf: BytesMut`
- [x] Implement `new()` and `with_capacity()` constructors
- [x] Implement `write_simple(s: &str)` — writes `+{s}\r\n`
- [x] Implement `write_bulk(data: &[u8])` — writes `${len}\r\n{data}\r\n`
- [x] Implement `write_int(n: i64)` — writes `:{n}\r\n`
- [x] Implement `write_array_header(len: usize)` — writes `*{len}\r\n`
- [x] Implement `write_array_value(v: &RedisValue)` — dispatches to correct write_* method
- [x] Implement `write_null_bulk()` — writes `$-1\r\n`
- [x] Implement `write_empty_array()` — writes `*0\r\n`
- [x] Implement `write_error(msg: &str)` — writes `-{msg}\r\n`
- [x] Implement `take()` — returns the buffer and starts a new empty one

## Verification

- All tests pass:
  - `test_write_simple_ok` — "OK" → "+OK\r\n"
  - `test_write_bulk_hello` — b"hello" → "$5\r\nhello\r\n"
  - `test_write_int_42` — 42 → ":42\r\n"
  - `test_write_array_header_3` — 3 → "*3\r\n"
  - `test_write_null_bulk` — `$-1\r\n`
  - `test_write_empty_array` — `*0\r\n`
  - `test_write_error_err_msg` — "ERR msg" → "-ERR msg\r\n"
  - `test_take_returns_and_resets` — take empty, write again, take again
- `cargo clippy` — zero warnings
- No may import anywhere in the codec module
