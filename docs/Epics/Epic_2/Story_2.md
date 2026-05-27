# Story 2.2 — RESPReader (decode direction)

**Objective:** Implement the RESPReader for decoding RESP wire format into RedisValue.

**Epic:** 2 — Codec Crate

**Dependencies:** Story 2.1

**Source docs:** `docs/05-protocol-layer-design.md`

## Code Anchors

- `crates/codec/src/lib.rs` — `pub struct RESPReader { buf: BytesMut, pos: usize }`
- `crates/codec/src/reader.rs` — implementation

## Struct

```rust
pub struct RESPReader {
    buf: BytesMut,
    pos: usize,
}
```

## Methods

```rust
impl RESPReader {
    pub fn new(buf: BytesMut) -> Self;
    pub fn read_value(&mut self) -> Result<RedisValue, RedisError>;
    fn read_line(&mut self) -> Result<&[u8], RedisError>;
    fn read_bytes(&mut self, n: usize) -> Result<&[u8], RedisError>;
    fn read_simple(&mut self) -> Result<RedisValue, RedisError>;
    fn read_error(&mut self) -> Result<RedisValue, RedisError>;
    fn read_integer(&mut self) -> Result<RedisValue, RedisError>;
    fn read_bulk(&mut self) -> Result<RedisValue, RedisError>;
    fn read_array(&mut self) -> Result<RedisValue, RedisError>;
}
```

## Tasks

1. Define `RESPReader` with `buf: BytesMut` and `pos: usize`
2. Implement `new(buf)` constructor
3. Implement `read_line()` — reads until `\r\n`, returns the line content
4. Implement `read_bytes(n)` — reads exactly N bytes
5. Implement `read_value()` — reads marker char, dispatches to correct read_* method
6. Implement `read_simple()` — reads `+{line}\r\n` → `SimpleString(line)`
7. Implement `read_error()` — reads `-{line}\r\n` → `Error(line)`
8. Implement `read_integer()` — reads `:{n}\r\n` → `Integer(n)`
9. Implement `read_bulk()` — reads `$-1\r\n` → `Null`, or `$N\r\n{N bytes}\r\n` → `BulkString(N bytes)`
10. Implement `read_array()` — reads `*N\r\n` → recursively decode N values → `Array(vec)`

## Verification

- `cargo test -p codec` — at least 8 unit tests:
  - `test_read_simple_ok` — "+OK\r\n" → SimpleString("OK")
  - `test_read_error_msg` — "-ERR msg\r\n" → Error("ERR msg")
  - `test_read_integer_42` — ":42\r\n" → Integer(42)
  - `test_read_bulk_string` — "$5\r\nhello\r\n" → BulkString(b"hello")
  - `test_read_null_bulk` — "$-1\r\n" → Null
  - `test_read_array_two_strings` — "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n" → Array([BulkString("foo"), BulkString("bar")])
  - `test_read_empty_array` — "*0\r\n" → Array([])
  - `test_read_nested_array` — "*1\r\n*2\r\n$3\r\na\r\n$3\r\nb\r\n" → Array([Array([...])])
- `cargo clippy -p codec` — zero warnings
