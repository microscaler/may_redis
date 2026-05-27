# Redis Protocol Analysis (RESP)

## Overview

Redis uses **RESP** (Redis Serialization Protocol), a simple text-based
protocol. Unlike PostgreSQL's binary wire protocol (which requires a full
library like `postgres-protocol`), RESP can be implemented in ~200 lines of
code because it's fundamentally line-oriented with simple type markers.

## RESP Wire Format

Each Redis protocol message is preceded by a type indicator:

```
+-------+----------+-----------------+
| Type  | Marker   | Example         |
+-------+----------+-----------------|
| Simple| $        | $4\r\nHello\r\n |
| Error | -        | -Error msg\r\n   |
| Integer| :       | :42\r\n          |
| Bulk   | $        | $5\r\nHello\r\n |
| Array  | *        | *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n |
+-------+----------+-----------------+
```

### Type Markers

| Marker | Name        | Payload                         |
|--------|-------------|---------------------------------|
| `$N`   | Bulk string | N bytes of data, terminated by \r\n |
| `*N`   | Array       | N elements followed by their types |
| `:N`   | Integer     | Decimal integer                 |
| `-$N`  | Error       | String error message            |
| `+$N`  | Simple      | Simple string (no binary safety)|
| `~$N`  | Arbitrary   | Arbitrary binary (RESP3 only)   |
| `=$N`  | Blob error  | Blob error (RESP3 only)         |
| `_`    | Null        | Null value                      |
| `,`    | Double      | Floating point number           |
| `%`    | Map         | Key-value pairs (RESP3)         |
| `>`    | Attribute   | Key-value attribute (RESP3)     |

### Simplified Model

For our use case (RESP2 compatibility), we only need:
- **Bulk strings** (`$N`) — command arguments and most responses
- **Arrays** (`*N`) — multi-bulk responses (e.g. `KEYS` returns array of keys)
- **Integers** (`:N`) — `SET` returns `:1`, `INCR` returns `:42`
- **Errors** (`-$N`) — `redis::RedisError`

## Command Flow

```
Client                          Redis Server
  |  SET key value EX 60        |
  |  $12\r\nSET key value EX 60\r\n |
  |                             |
  |  :1\r\n                      |
  |<----------------------------|
```

### Encoding Commands

Each command is a bulk array:
```
*<argcount>\r\n
$<len1>\r\n<arg1>\r\n
$<len2>\r\n<arg2>\r\n
...
```

Example: `SET foo bar EX 60`
```
*5\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$2\r\nbar\r\n$2\r\nEX\r\n$2\r\n60\r\n
```

### Decoding Responses

Each response starts with a type marker, followed by payload:
- Read the marker character
- Read the length (until `\r\n`)
- If negative: special value (null, empty array, etc.)
- If non-negative: read N bytes of payload

## Comparison with PostgreSQL Wire Protocol

| Aspect            | PostgreSQL Wire            | RESP (Redis)              |
|-------------------|----------------------------|---------------------------|
| Format            | Binary with message headers| Text-based, line-oriented |
| Framing           | Fixed-size headers         | Length-prefixed           |
| Parameter encoding| Type-aware binary          | ASCII bytes               |
| Streaming         | RowStream with columns     | Single response per cmd   |
| Auth              | SASL handshake             | Simple AUTH command       |
| Transactions      | BEGIN/COMMIT protocol      | MULTI/EXEC (application)  |
| Prep stmts        | Parse/Bind/Execute/DDescribe| None (inline commands)  |

## Conclusion

RESP is significantly simpler than PostgreSQL's wire protocol. We can
implement a full RESP codec in ~150 lines (vs ~300 for postgres-protocol
adapter), and the client layer is much simpler because there are no
preparedStatement lifecycle, no binary type encoding, no COPY protocol.

The main complexity is in the **connection loop** — same pattern as
may_postgres: an epoll-based coroutine that handles bidirectional I/O
and dispatches responses via spsc channels.
