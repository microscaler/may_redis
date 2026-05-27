# Story 2.3 — Full RESP2 coverage and roundtrip tests

**Objective:** Complete coverage of all RESP2 type markers and add roundtrip encode→decode tests.

**Epic:** 2 — Codec Crate

**Dependencies:** Story 2.2

**Source docs:** `docs/01-protocol-analysis.md`, `docs/05-protocol-layer-design.md`

## Code Anchors

- `crates/codec/src/reader.rs` — additional edge cases
- `crates/codec/src/roundtrip.rs` — encode→decode verification

## RESP2 Type Markers (in scope)

| Marker | Name | Payload |
|--------|------|---------|
| `+$N` | Simple string | N bytes, no binary safety |
| `-$N` | Error | String error message |
| `:N` | Integer | Decimal integer |
| `$N` | Bulk string | N bytes terminated by \r\n |
| `*N` | Array | N elements followed by their types |
| `$-1` | Null bulk string | Null value |
| `*0\r\n` | Empty array | Zero elements |

## Tasks

1. Add `read_value()` test for empty bulk string `$0\r\n\r\n` → `BulkString(b"")`
2. Add roundtrip test for simple string: `write_simple("OK") → read_value() == SimpleString("OK")`
3. Add roundtrip test for bulk string: `write_bulk(b"hello") → read_value() == BulkString(b"hello")`
4. Add roundtrip test for integer: `write_int(42) → read_value() == Integer(42)`
5. Add roundtrip test for array: `write_array_value(SET, key, value) → read_value() == Array([BulkString("SET"), BulkString("key"), BulkString("value")])`
6. Add roundtrip test for SET command encoding: `cmd SET key value EX 60 → wire bytes → decode → verify Array(["SET", "key", "value", "EX", "60"])`
7. Add roundtrip test for KEYS response: decode `*2\r\n$8\r\nuser:1\r\n$8\r\nuser:2\r\n` → verify `Array([BulkString("user:1"), BulkString("user:2")])`
8. Add edge case: large payload (64KB) encoding and decoding

## Verification

- `cargo test -p codec` — at least 15 total unit tests
- All roundtrip tests pass: encode → decode → compare
- `cargo clippy -p codec` — zero warnings
- `cargo doc -p codec` — all public items documented
