# RESP Protocol Reference

- Status: unverified
- Source docs: `docs/01-protocol-analysis.md`, `docs/05-protocol-layer-design.md`
- Code anchors: `src/codec/`
- Last updated: 2026-05-27

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
| `$-1` | Null bulk string | Null value |
| `*0\r\n` | Empty array | Zero elements |

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
| `$-1` | `Option<String>` | Null bulk string |
| `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` | `Vec<String>` | Array of strings |
| `*0\r\n` | `Vec<String>` | Empty array |
| `-ERR msg\r\n` | `RedisError` | Error |

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
