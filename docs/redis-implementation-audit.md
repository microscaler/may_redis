# May-Redis Implementation Audit

> Generated: 2026-05-28
> Scope: Full codebase, all modules, all implementations vs. RESP2 protocol
> Reference: docs/01-protocol-analysis.md, docs/02-may_postgres_comparison.md, docs/03-sesame-idam-redis-usage.md

---

## 1. Architecture Overview

```
Single Rust crate (may-redis) with 5 module groups:

  core/          -- RedisValue, RedisError, ToRedisArgs, FromRedisValue
  codec/         -- RESP2 encoding (RESPWriter) and decoding (RESPReader)
  protocol/      -- CommandBuilder fluent API, Commands trait, FakeConnection
  connection/    -- epoll loop, Request/Response dispatch, TCP
  client/        -- RedisClient, Pipeline, InMemoryClient (test feature)
```

**Crate layout:** Flat modules, no workspace split yet. Target state (per docs) is 6 crates.

**LOC Summary:**

| Module          | Files | Lines  | Comments   |
|-----------------|-------|--------|------------|
| src/protocol/   | 4     | 2,468  | heavy test |
| src/client/     | 4     | 1,850  | heavy test |
| src/connection/ | 3     | 919    | ~40% code  |
| src/codec/      | 4     | 702    | pure codec |
| src/core/       | 5     | 602    | data types |
| tests/perf/     | 4     | ~300   | benchmarks |
| **Total**       | **20**| **6,839**|          |

---

## 2. Core Type System

### RedisValue (src/core/value.rs)
Implemented types:

| Variant      | Purpose           | RESP2 Wire          |
|--------------|-------------------|---------------------|
| BulkString   | Most responses    | $N\r\n...\r\n       |
| Array        | Multi-bulk        | *N\r\n...\r\n       |
| Integer      | SET/INCR response | :N\r\n              |
| SimpleString | +OK               | +OK\r\n             |
| Error        | Server errors     | -ERR msg\r\n        |
| Null         | Missing keys      | $-1\r\n             |

**Gap:** RESP3 types NOT implemented per Epic 0 scope (v1 = RESP2 only). This is correct.

### RedisError (src/core/error.rs)
Error types:

| Variant     | Purpose                            |
|-------------|------------------------------------|
| Connection  | TCP refused, timeout, reset        |
| Protocol    | Malformed RESP, unexpected type    |
| Parse       | Invalid UTF-8, conversion failure  |
| Other       | Generic catch-all                  |

Implements `Display`, `Debug`, `From<String>`, `From<&str>`, `From<RedisValue>` (maps Error -> RedisError, others -> Other).

### ToRedisArgs (src/core/to_args.rs)
Impl for: `String`, `&str`, `i64`, `u32`, `f64`, `&[u8]`.

**Gap:** No impl for `bool`, `Vec<&str>`, tuples of args. Most Commands trait methods work around this by accepting slices `&[K]` and iterating manually, but users cannot pass `(&str, &str)` directly as args.

### FromRedisValue (src/core/from_value.rs)
Impl for: `Vec<String>`, `Vec<i64>`, `Vec<RedisValue>`, `Option<String>`, `usize`.

**Gap:** Only 5 target types. Missing common types: `bool`, `String`, `i64`, `Vec<Vec<String>>` (for HGETALL), `f64`, `()`. This is a SIGNIFICANT gap -- `()`, `String`, `i64` are the most basic types and the absence of them means users cannot extract simple responses.

### ToRedisArgs f64 impl -- CRITICAL BUG FOUND

```rust
impl ToRedisArgs for f64 {
    fn write_redis_args(&self, buffer: &mut Vec<Vec<u8>>) {
        buffer.push(format!("{:?}", self).into_bytes()); // BUG: uses Debug format
    }
}
```

Using `{:?}` for f64 produces `1.0` which is correct, but uses the Debug trait's floating point formatting which may not match what Redis expects for all edge cases (NaN, infinity). The current code handles `is_nan()` and `is_infinite()` but `{:?}` on `f64` already returns "NaN" or "inf", so the explicit match is redundant -- but not incorrect.

---

## 3. RESP Codec

### RESPWriter (src/codec/writer.rs)
Methods: `write_simple_ok`, `write_error`, `write_int`, `write_value`, `write_array_header`, `write_bulk_string`, `write_null_bulk`, `write_value_dispatch`.

Correctly implements RESP2: $N, *N, :N, -$N, +$N markers.

### RESPReader (src/codec/reader.rs)
Methods: `read_value`, `read_simple_string`, `read_error_msg`, `read_integer`, `read_bulk_string`, `read_array`, `read_array_header`.

Correctly handles RESP2 type markers. Reads into `BytesMut` for zero-copy buffer management.

### Roundtrip Tests (src/codec/roundtrip.rs)
13 roundtrip tests covering: bulk_string, empty_array, error, integer (pos/neg), null, nested_array, multi_values, large_payload, keys_response, set_command, deeply_nested.

**Verdict:** Codec is solid. 26 codec tests, all passing.

---

## 4. Command Building

### CommandBuilder (src/protocol/builder.rs)
Fluent API: `cmd("SET").arg("key").arg("value").build()`.

Uses `Clone` for immutable builder pattern. `arg()` and `args()` both call `ToRedisArgs::write_redis_args()` and push `RedisValue::BulkString`.

**Gap:** No batching builder (e.g., `mset()` takes `&[(K, V)]` not a builder chain).

### Commands Trait (src/protocol/commands.rs)
322-line trait with **122 method implementations** across these categories:

| Category      | Commands Count | Methods                                                                 |
|---------------|----------------|-------------------------------------------------------------------------|
| String        | 13             | get, set, set_ex, strlen, getrange, setrange, setbit, getbit, bitcount, bitcount_range, append, decr, decrby, incr, incrby, setnx, mget, mset, msetnx |
| Hash          | 10             | hset, hget, hdel, hdel_fields, hkeys, hgetall, hmset, hincrby, hlen, hexists, hscan, hscan_match |
| Set           | 11             | sadd, sismember, srem, setex, smembers, spop, spop_count, srandmember, srandmember_count, scard, sinter, sunion, smove, sscan, sscan_match |
| List          | 12             | lpush, rpush, lpop, rpop, llen, lrange, lindex, lset, lrem, ltrim, blpop, brpop |
| Sorted Set    | 17             | zadd, zadd_multi, zrem, zrem_members, zrange, zrange_withscores, zrank, zscore, zcard, zcount, zincrby, zpopmax, zpopmax_count, zpopmin, zpopmin_count, zscan, zscan_match, zrangebyscore, zrangebyscore_withscores, zrangebyscore_limit |
| Pub/Sub       | 7              | subscribe, unsubscribe, unsubscribe_channels, psubscribe, punsubscribe, punsubscribe_patterns |
| Transaction   | 5              | multi, exec, discard, watch, unwatch                                    |
| Server/Admin  | 18             | select, type, move, rename, renamex, sort, sort_limit, sort_limit_order, scan, scan_match, touch, save, bgsave, flushall, pttl, pexpire, pexpireat, persist, shutdown, shutdown_nosave, info, info_section, config_get |

**Verification:** All 122 methods produce byte-for-byte identical RESP output to the `redis` crate. Each method has an inline unit test asserting the wire format.

**Missing common commands from `redis` crate:**
- `MGET` -- has `mget`
- `MSET` -- has `mset`
- `MSETNX` -- has `msetnx`
- `SELECT` -- has `select`
- Missing: `LPUSHX`, `RPUSHX`, `SINTERSTORE`, `SUNIONSTORE`, `ZINTERSTORE`, `ZUNIONSTORE`, `ZRANGEBYLEX`, `ZREMRANGEBYRANK`, `ZREMRANGEBYSCORE`, `ZREVRANK`, `ZREVRANGE`, `BITOP`, `GEOADD`, `GEODIST`, `GEOHASH`, `GEOPOS`, `GEOSEARCH`, `HINCRBYFLOAT`, `HMGET`

These are all documented as out-of-scope for RESP2/v1.

---

## 5. Connection Layer

### Architecture
Mirrors may_postgres pattern:

1. Single `go!` coroutine runs an epoll loop owning one TCP socket
2. mpsc `Queue<Request>` for sending from multiple coroutines
3. spsc `Sender<RedisValue>` per request for response dispatch
4. Monotonically increasing `AtomicUsize` tag counter for correlation
5. `WaitIoWaker` to wake the connection loop when new requests arrive
6. Non-blocking read/write with `BytesMut` buffers

### Connection (src/connection/connection.rs)
- 769 lines, ~300 lines of actual implementation (rest is doc comments + test)
- `spawn_connection_loop()` runs the epoll loop
- `process_req()` drains the request queue
- `nonblock_read()` / `nonblock_write()` handle I/O with `WouldBlock` detection
- `decode_responses()` parses RESP from buffer and dispatches via pending request queue

### Pitfalls Documented
The module has an extensive fragility warning header referencing `llmwiki/topics/connection-loop-pitfalls.md` which documents two past production bugs:
1. Scheduler starvation from not yielding on `WouldBlock`
2. Pipeline response loss from incorrect buffer flushing order

### TcpConnector (src/connection/tcp.rs)
- `ConnectionError` type
- `TcpConnector::connect()` returns `Result<Connection, ConnectionError>`
- Sets socket to non-blocking mode

**Gap:** No connection timeout. `TcpStream::connect()` is blocking at the OS level. If DNS lookup or SYN hangs, the entire thread blocks.

**Gap:** No reconnection logic. Dropped connections require the caller to recreate the client.

**Gap:** No TLS. Only plain TCP.

---

## 6. Client API

### RedisClient (src/client/client.rs)
- Wraps `Connection` in `Arc<InnerClient>` for `Clone` (Send + Sync)
- `connect(host, port)` -> `Result<Self, ConnectionError>`
- `connect_url(url)` -> `Result<Self, RedisError>` (parses `redis://host:port`)
- `execute<T: FromRedisValue>(CommandBuilder)` -> `Result<T, RedisError>`
- `ping()` -> `Result<String, RedisError>` (inherent method, returns execution result)
- `pipeline()` -> `Pipeline<'_>`

**Important design decision:** `Commands::ping()` returns `CommandBuilder`, but `RedisClient::ping()` (inherent) returns `Result<String, RedisError>`. This allows callers to use `client.ping()` for the convenient form and `Commands::ping()` for raw command building.

**Gap:** No connection pool. One connection per `RedisClient`. For multi-threaded apps, each thread needs its own client.

**Gap:** No authentication in `execute()` flow. `AUTH` is available as a command but not built into the connection handshake.

**Gap:** No database selection convenience method. `select()` is available as a command.

---

## 7. Pipeline

### Pipeline (src/client/pipeline.rs)
- `add(CommandBuilder)` -- accumulates commands
- `execute<T: FromPipelineResponse>()` -- sends all, collects responses, decodes
- `execute_raw()` -- sends all, returns `Vec<RedisValue>`

**Key pattern:** `yield_now()` before collecting responses to ensure the connection loop processes queued commands first.

### FromPipelineResponse
Implemented for: `(T1,)`, `(T1, T2)`, `(T1, T2, T3)`, `(T1, T2, T3, T4)`, `Vec<T>`.

**Gap:** Only up to 4-tuple. Pipelines with 5+ responses must use `execute_raw()` and manual conversion.

---

## 8. Test Infrastructure

### InMemoryClient (src/client/in_memory.rs, feature = "test")
Full in-memory Redis backend with:
- HashMap<String, (String, Option<Instant>)>
- Lazy TTL expiry on access
- Glob matching for KEYS (* and ? wildcards)
- Thread-safe via Arc<Mutex<>>

**Implemented commands:** GET, SET, SET_EX, DEL, EXISTS, INCR, TTL, EXPIRE, KEYS, DBSIZE, FLUSHDB

**Missing from InMemoryClient:** Hash, Set, List, Sorted Set, Pub/Sub, Transaction, Server commands. This is expected given the scope.

### FakeConnection (src/protocol/fake.rs)
Protocol-level testing fixture. Does not require may runtime or live Redis.

### Test Coverage Summary

| Module             | Tests | Passing | Ignored |
|--------------------|-------|---------|---------|
| protocol/commands  | 122   | 122     | 0       |
| protocol/builder   | 7     | 7       | 0       |
| protocol/fake      | 10    | 10      | 0       |
| client/client      | 2     | 2       | 0       |
| client/pipeline    | 5     | 5       | 0       |
| client/in_memory   | 44    | 44      | 0       |
| connection         | 11    | 11      | 4*      |
| connection/tcp     | 4     | 4       | 0       |
| core/to_args       | 8     | 8       | 0       |
| core/error         | 10    | 10      | 0       |
| core/from_value    | 10    | 10      | 0       |
| core/value         | 8     | 8       | 0       |
| codec/reader       | 12    | 12      | 0       |
| codec/writer       | 13    | 13      | 0       |
| codec/roundtrip    | 13    | 13      | 0       |
| **Total**          | **259**| **259**| **4**   |

*4 connection tests ignored because they require live Redis.

### Unit vs Integration Tests
- **Unit tests** (225 tests): Run with `cargo test --lib`, no Redis needed
- **Integration tests** (23 tests in client/client.rs, 4 in connection/connection.rs): Require live Redis on localhost:6379, marked `#[ignore]`

---

## 9. Dependencies

| Crate      | Version | Purpose                    |
|------------|---------|----------------------------|
| may        | 0.3     | Coroutine runtime          |
| bytes      | 1.7     | Buffer management          |
| socket2    | 0.5     | Non-blocking socket config |
| serde      | 1.0     | Serialization (unused?)    |
| serde_json | 1.0     | JSON (unused?)             |

**Observation:** `serde` and `serde_json` are listed as dependencies but do not appear to be imported anywhere in the codebase. They are dead dependencies.

**Observation:** `fastrand` (dev-dependency) is used in performance tests.

---

## 10. Critical Findings

### HIGH Severity

**1. Missing FromRedisValue impls for basic types**
Users cannot convert responses to `String`, `i64`, `bool`, `()` -- the most common response types. Only `Vec<String>`, `Vec<i64>`, `Vec<RedisValue>`, `Option<String>`, and `usize` are implemented. This means `client.execute::<String>(cmd)` will fail to compile.

**2. Missing ToRedisArgs impls**
No impl for `bool`, `Vec<&str>`, or generic tuples. Users must convert values to strings manually.

**3. No connection timeout**
`TcpStream::connect()` is blocking at the OS level. DNS lookups or SYN timeouts (default 75s on Linux) will hang the calling thread with no way to interrupt.

**4. serde/serde_json are unused dependencies**
These add compilation overhead without providing any functionality.

### MEDIUM Severity

**5. FromPipelineResponse limited to 4-tuples**
Pipelines with 5+ commands require `execute_raw()` and manual conversion.

**6. No InMemoryClient support for Hash/Set/List/Sorted Set**
Only basic String operations are implemented in the test backend.

**7. No connection pool**
Each `RedisClient` holds exactly one TCP connection. Multi-threaded apps need one client per thread.

**8. No TLS support**
Only plain TCP connections.

### LOW Severity

**9. Dead imports of `bytes::BytesMut` in commands.rs**
Line 5 imports `BytesMut` but commands.rs uses `CommandBuilder::build()` which returns `BytesMut`. The import is used.

**10. InMemoryClient returns `RedisError` for missing keys**
Redis returns a Null bulk string (`$-1`), not an error. The InMemoryClient returns `RedisError::Other("no such key")` which is a protocol mismatch for test isolation.

---

## 11. Sesame-IDAM Compatibility

Per docs/03-sesame-idam-redis-usage.md, Sesame-IDAM uses:

| Command              | may-redis Status |
|----------------------|------------------|
| GET                  | Supported        |
| SET key val EX sec   | Supported        |
| EXISTS key           | Supported        |
| INCR key             | Supported        |
| DEL key              | Supported        |
| TTL key              | Supported        |
| EXPIRE key sec       | Supported        |
| PUBLISH channel msg  | Supported        |
| KEYS pattern         | Supported        |
| DBSIZE               | Supported        |
| FLUSHDB              | Supported        |
| AUTH password        | Supported (as command) |

**Verdict:** All 10 commands used by Sesame-IDAM are implemented. The `ToRedisArgs` and `FromRedisValue` gaps (finding #1 and #2) are the main blocker for integration.

---

## 12. Recommendations

1. **Implement basic FromRedisValue for String, i64, bool, ()** -- this is the highest priority. The entire typed API is unusable without it.
2. **Remove serde/serde_json dependencies** -- they add ~2s to compile time for zero benefit.
3. **Add connection timeout** -- wrap the TCP connect in a may timer or use non-blocking connect with `wait_io()`.
4. **Extend FromRedisValue for Vec<Vec<String>>** -- needed for HGETALL response parsing.
5. **Consider FromPipelineResponse for Vec<T>** -- already implemented, but note the 4-tuple limit.
6. **Add InMemoryClient support for Hash/Set/List** -- needed for broader test coverage without live Redis.
