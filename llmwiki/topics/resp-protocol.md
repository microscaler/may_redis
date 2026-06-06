# RESP Protocol Reference

- Status: verified
- Source docs: `docs/01-protocol-analysis.md`, `docs/05-protocol-layer-design.md`
- Code anchors: `src/codec/`, `src/core/from_value.rs`
- Last updated: 2026-06-05

## RESP Wire Format

Redis uses **RESP** (Redis Serialization Protocol), a simple text-based protocol. Unlike PostgreSQL's binary wire protocol, RESP can be implemented in ~200 lines of code because it's fundamentally line-oriented with simple type markers.

### RESP2 Type Markers (in scope)

| Marker | Name | Payload |
|--------|------|---------|
| `+$N` | Simple string | N bytes, no binary safety |
| `-$N` | Error | String error message |
| `:N` | Integer | Decimal integer |
| `$N` | Bulk string | N bytes terminated by `\r\n` |
| `*N` | Array | N elements followed by their types |
| `$-1` | Null bulk string | Null value (no payload) |
| `*-1` | Null array | Null value (no elements) — **not** the same as `$-1` |
| `*0\r\n` | Empty array | Zero elements (valid empty array, not null) |

### RESP3 Types (out of scope for v1)

| Marker | Name | Payload |
|--------|------|---------|
| `~$N` | Arbitrary binary | N bytes (RESP3 only) |
| `=$N` | Blob error | N bytes (RESP3 only) |
| `_` | Null | Null value |
| `,` | Double | Floating point number |
| `%` | Map | Key-value pairs |
| `>` | Attribute | Key-value attribute |

### Command Encoding

Each Redis command is a bulk array of bulk strings:

```
*<argcount>\r\n
$<len1>\r\n<arg1>\r\n
$<len2>\r\n<arg2>\r\n
...
```

Example: `SET foo bar EX 60`

```
*5\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n$2\r\nEX\r\n$2\r\n60\r\n
```

### Response Decoding

1. Read the marker character
2. Read the length (until `\r\n`)
3. If negative: special value (null, empty array, etc.)
4. If non-negative: read N bytes of payload

### Type Mapping (RESP → Rust)

| RESP Type | Rust Type | Example |
|-----------|-----------|---------|
| `+OK` | `Result<(), E>` | Simple string |
| `:42` | `i64` | Integer |
| `$5\r\nhello\r\n` | `String` | Bulk string |
| `$-1` | `Option<String>` / `Option<T>` | Null bulk string |
| `*-1` | `Option<Vec<T>>` | Null array — Redis uses this for aborted `EXEC` (WATCH conflict) |
| `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` | `Vec<String>` | Array of strings |
| `*0\r\n` | `Vec<String>` | Empty array |
| `-ERR msg\r\n` | `RedisError` | Error |

### SCAN-family cursor encoding

Redis 7 returns SCAN / HSCAN / SSCAN / ZSCAN cursors as **bulk strings**
(e.g. `"0"`, `"44"`), not integers, even in RESP2. The first element of
the `*2` array is `$1\r\n0\r\n` or similar — not `:0\r\n`.

`FromRedisValue for (i64, Vec<String>)` (and the `(i64, Vec<(String, f64)>)`
variant) must parse the cursor via `scan_cursor()` in
`src/core/from_value.rs`, accepting `Integer`, `BulkString`, or
`SimpleString`.

Integration tests that assumed `:N` integer cursors failed with parse
errors until this was fixed (2026-06-05).

## Implementation

Located in `src/codec/`:

- `RESPWriter` — writes RESP commands into a `BytesMut`
- `RESPReader` — reads RESP responses from a `BytesMut`
- `encode_command()` — converts `RedisValue` array into RESP wire format
- `decode_response()` — converts RESP wire format into `RedisValue`

## Comparison with PostgreSQL Wire Protocol

| Aspect | PostgreSQL Wire | RESP (Redis) |
|--------|----------------|--------------|
| Format | Binary with message headers | Text-based, line-oriented |
| Framing | Fixed-size headers | Length-prefixed |
| Parameter encoding | Type-aware binary | ASCII bytes |
| Streaming | RowStream with columns | Single response per cmd |
| Auth | SASL handshake | Simple AUTH command |
| Transactions | BEGIN/COMMIT protocol | MULTI/EXEC (application) |
| Prep stmts | Parse/Bind/Execute/DDescribe | None (inline commands) |
