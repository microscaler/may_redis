# PRD: File Size Audit & Breakdown Plan

## Goal
Break down all files exceeding 350 lines into smaller, focused modules.

## Current State (post-connection split + client test fixes)

**Total: 10,996 lines across 37 files.**
**356 tests pass. 0 compilation errors.**

### Production files over 350 lines: 8

| # | File | Lines | Structs | Impl blocks | Fns |
|---|------|-------|---------|-------------|-----|
| 1 | `protocol/commands_tests.rs` | 875 | 0 | 0 | 123 (all tests) |
| 2 | `client/client_tests.rs` | 607 | 0 | 0 | 38 (all tests) |
| 3 | `client/client.rs` | 557 | 3 | 11 | 13 |
| 4 | `tls/mod.rs` | 552 | 5 | 12 | 26 (+56 tests) |
| 5 | `connection/connection_tests.rs` | 489 | 0 | 0 | 19 (all tests) |
| 6 | `connection/tcp.rs` | 477 | 2 | 6 | 25 (+100 tests) |
| 7 | `connection/connection.rs` | 404 | 2 | 5 | 6 |
| 8 | `connection/connection_io.rs` | 368 | 0 | 0 | 3 |

### Production files with embedded tests over 350 lines: 5

| File | Total | Prod | Tests | Split point |
|------|-------|------|-------|-------------|
| `codec/reader.rs` | 820 | 342 | 478 | #[cfg(test)] at line 343 |
| `client/in_memory.rs` | 754 | 344 | 410 | #[cfg(test)] at line 345 |
| `protocol/builder.rs` | 564 | 293 | 271 | #[cfg(test)] at line 294 |
| `core/from_value.rs` | 599 | 160 | 439 | #[cfg(test)] at line 161 |
| `codec/roundtrip.rs` | 510 | 23 | 487 | #[cfg(test)] at line 24 |

### Production files under 350 lines: 22 (OK)

```
client/pipeline.rs             318  protocol/commands/admin.rs     230
core/to_args.rs                327  protocol/fake.rs               235
protocol/commands/strings.rs   217  core/error_tests.rs            220
protocol/commands/sorted_sets.rs 216 core/from_value.rs             160 (prod only)
protocol/commands/hashes.rs    127  connection/test_limits.rs       134
protocol/commands/sets.rs      130  core/error.rs                   138
protocol/commands/lists.rs     130  protocol/fake_tests.rs          119
protocol/commands/pubsub.rs     95  codec/writer.rs                 109
core/value.rs                   107 protocol/mod.rs                    32
connection/mod.rs                61  client/mod.rs                      46
protocol/commands/mod.rs         54  core/mod.rs                         42
protocol/commands/transactions.rs 50  codec/mod.rs                        36
lib.rs                            35  lib.rs                              35
```

---

## PRIORITY 0: Extract all test-only files (zero risk)

Files where the ENTIRE file is tests with no production code:

### 0a. `protocol/commands_tests.rs` (875 lines, 123 test fns)

Already a test module under `protocol/mod.rs`. Split by domain:

```
protocol/commands_tests.rs       (875 lines)
  -> commands_strings_tests.rs  (~100 lines): GET, SET, DEL, INCR, TTL, EXPIRE, PUBLISH, KEYS, DBSIZE, FLUSHDB, PING, AUTH, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, APPEND, DECR, DECRBY, SETNX, MGET, MSET, MSETNX
  -> commands_hashes_tests.rs   (~100 lines): HSET, HGET, HDEL, HKEYS, HGETALL, HMSET, HINCRBY, HLEN, HEXISTS, HSCAN
  -> commands_sets_tests.rs     (~100 lines): SADD, SISMEMBER, SREM, SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION, SMOVE, SSCAN
  -> commands_lists_tests.rs    (~80 lines): LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP
  -> commands_sorted_sets_tests.rs (~100 lines): ZADD, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZSCAN
  -> commands_pubsub_tests.rs   (~70 lines): SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE
  -> commands_transactions_tests.rs (~60 lines): MULTI, EXEC, DISCARD, WATCH, UNWATCH, SELECT
  -> commands_admin_tests.rs    (~150 lines): SCAN, TOUCH, SAVE, BGSAVE, FLUSHALL, PTTL, PEXPIRE, PEXPIREAT, PERSIST, SHUTDOWN, INFO, CONFIG GET, MOVE, RENAME, RENAMENX, SORT, TYPE
```

Result: 8 files x ~70-150 lines each.

### 0b. `client/client_tests.rs` (607 lines, 38 fns)

Already a test module under `client/mod.rs`. Split into unit + integration:

```
client/client_tests.rs           (607 lines)
  -> client_tests.rs             (~180 lines): test_redis_client_struct, test_commands_trait_methods_exist, init_may_runtime, shared_client, run_may helper
  -> client_integration_tests.rs (~420 lines): all test_integration_* functions (ping, set_get, incr, exists_del, dbsize, set_ex_ttl, keys, send_sync_clone, error_propagation, pipeline, concurrent variants, request_ordering, response_correlation, server_error, wrong_type, empty_pipeline, null_response, redis_server_error_value, set_get_ex, del, expire, publish)
```

Result: 2 files, 180 + 420 lines.

### 0c. `connection/connection_tests.rs` (489 lines, 19 fns)

Already a test module under `connection/mod.rs`. Can stay as-is (489 is the test module, production code is already split). Or split:

```
connection/connection_tests.rs   (489 lines)
  -> connection_tests.rs         (~200 lines): test_request_new, test_pending_request, test_process_req_moves_to_write_buf, test_process_req_multiple, test_decode_responses_*
  -> connection_lifecycle_tests.rs (~280 lines): test_connection_connect, test_connection_send_tags, test_connection_id, test_connection_drop variants
```

Result: Optional. Can stay as-is since it is a test module.

---

## PRIORITY 1: Extract embedded tests from production files

### 1a. `codec/roundtrip.rs` (510 lines, PROD=23, TESTS=487) — BEST WIN

Only 23 lines of production code (just the `roundtrip()` helper). 45 test functions under `#[cfg(test)]`.

```
codec/roundtrip.rs               (23 lines)   -> keep (just the helper)
codec/roundtrip_tests.rs         (487 lines)  -> extract all tests
```

Wire: `codec/mod.rs` add `#[cfg(test)] mod roundtrip_tests;`

Result: 2 files. Production is 23 lines. Cleanest win in the codebase.

### 1b. `core/from_value.rs` (599 lines, PROD=160, TESTS=439)

10 impl blocks for FromRedisValue (Vec<i64>, Vec<String>, Option<String>, usize, u64, i32, u8, f64). 439 lines of tests under `#[cfg(test)]`.

```
core/from_value.rs               (160 lines) -> keep (all 10 impl blocks)
core/from_value_tests.rs         (439 lines) -> extract tests
```

Result: 2 files, both under 350.

### 1c. `codec/reader.rs` (820 lines, PROD=342, TESTS=478)

1 struct (RESPReader), 1 Drop impl, 3 impl blocks, 13 fns in prod. Tests at line 343.

```
codec/reader.rs                  (342 lines) -> already under 350, keep
codec/reader_tests.rs            (478 lines) -> extract tests
```

Result: reader.rs stays at 342 (safe). Test file extracted.

### 1d. `client/in_memory.rs` (754 lines, PROD=344, TESTS=410)

2 structs (InMemoryStore, InMemoryClient), 4 impl blocks, 28 production fns.

```
client/in_memory.rs              (344 lines) -> already under 350, keep
client/in_memory_tests.rs        (410 lines) -> extract tests
```

Result: in_memory.rs stays at 344 (safe).

### 1e. `protocol/builder.rs` (564 lines, PROD=293, TESTS=271)

1 struct (CommandBuilder), 2 impl blocks, 13 production fns.

```
protocol/builder.rs              (293 lines) -> already under 350, keep
protocol/builder_tests.rs        (271 lines) -> extract tests
```

Result: both files under 350.

---

## PRIORITY 2: Split remaining production files >350

### 2a. `tls/mod.rs` (552 lines, PROD=496, TESTS=56)

5 structs, 12 impl blocks. The TLS layer is dense because of rustls trait impls (ServerCertVerifier alone is ~35 lines). Tests at end (line 497).

```
tls/mod.rs                       (20 lines)   -> module file, re-exports only
tls/config.rs                    (~200 lines) -> TlsVersion, TlsConfig, ClientCerts, RustlsRootCerts
tls/connector.rs                 (~220 lines) -> TlsConnector, TlsStream, SkipVerifier, TLS handshake
tls/tests.rs                     (~56 lines)  -> extracted tests
```

Structure of config.rs:
- TlsVersion enum + impl (from_str, to_supported) - ~50 lines
- RustlsRootCerts enum + Default + into_config impl - ~80 lines
- ClientCerts struct + from_pem/from_der + Default impl - ~70 lines

Structure of connector.rs:
- TlsError enum + Display + Error impl - ~30 lines
- SkipVerifier struct + rustls::client::danger::ServerCertVerifier impl - ~40 lines
- TlsConfig struct + Default + into_config - ~60 lines
- TlsStream struct + inner_mut + inner - ~25 lines
- Read impl + Write impl for TlsStream - ~20 lines
- TlsConnector + handshake - ~60 lines

Result: 4 files, all under 250 lines.

### 2b. `client/client.rs` (557 lines, PROD=557, NO tests)

3 structs (TimeoutGuard, InnerClient, RedisClient), 11 impl blocks, 13 fns.

This is the hardest one. No tests embedded. Heavy doc comments.

```
client/client.rs                 (~220 lines) -> RedisClient struct + core connection methods
client_timeout.rs                (~140 lines) -> TimeoutGuard + execute_with_timeout + execute_timeout
client_url.rs                    (~100 lines) -> url_decode helper + connect_url
client_commands.rs               (~40 lines)  -> 8 domain trait impls + ping + pipeline
```

Structure of client.rs:
- imports (lines 1-10) - ~10 lines
- InnerClient struct + docs - ~10 lines
- RedisClient struct + docs - ~5 lines
- impl RedisClient::connect, connect_with_timeout, connect_with_ssrf_protection - ~80 lines
- impl RedisClient::command_policy, execute (inherent methods) - ~40 lines
- Empty domain trait impls + closing docs - ~15 lines

Structure of client_timeout.rs:
- TimeoutGuard struct + docs - ~20 lines
- TimeoutGuard::new + Drop impl - ~15 lines
- impl RedisClient::execute_with_timeout - ~80 lines (biggest function, ~80 lines of timeout logic)
- impl RedisClient::execute_timeout - ~10 lines

Structure of client_url.rs:
- url_decode helper - ~10 lines
- impl RedisClient::connect_url - ~90 lines (URL parsing, IPv6, password decode, AUTH)

Structure of client_commands.rs:
- impl StringsCommands for RedisClient {} - 1 line
- impl HashesCommands for RedisClient {} - 1 line
- ... (6 more domain traits)
- ping method - ~12 lines
- pipeline method - ~3 lines
- Closing doc comment - ~5 lines

Result: 4 files x 40-220 lines.

### 2c. `connection/tcp.rs` (477 lines, PROD=377, TESTS=100)

2 structs (SsrfConfig, TcpConnector), 6 impl blocks. Tests at line 378.

```
connection/tcp.rs                (~277 lines) -> ConnectionError, ssrf_allowed, is_blocked helpers, SsrfConfig, TcpConnector, connect methods
connection/tcp_tests.rs          (~100 lines) -> extract 100 lines of tests
```

Structure of tcp.rs:
- ConnectionError enum + Display + Error impl - ~30 lines
- SsrfConfig struct + Default impl - ~15 lines
- ssrf_allowed, is_blocked, is_blocked_v4, is_blocked_v6 - ~120 lines
- TcpConnector + connect/connect_with_ssrf_check/connect_with_timeout/connect_timeout/connect_url/connect_url_timeout/resolve/connect_addr_with_timeout - ~120 lines

Result: 277 + 100 = both under 350.

### 2d. `connection/connection.rs` (404 lines, PROD=404)

2 structs (Request, Connection), 5 impl blocks (std::fmt, std::error, Drop, Connection). 6 fns.

Heavy doc comments inflate the file. Actual logic is small.

```
connection/connection.rs         (~280 lines) -> Connection struct + impl + Request struct
connection_limits.rs             (~120 lines) -> ConnectionLimitError + connect_with_limits + Display/Error impls
```

Structure of connection.rs:
- Module doc comment - ~30 lines
- imports - ~15 lines
- Request struct + impl - ~40 lines
- Connection struct + impl (connect, connect_with_ssrf_protection, send, Drop) - ~200 lines

Structure of connection_limits.rs:
- ConnectionLimitError struct + Display/Error impls - ~20 lines
- connect_with_limits fn + docs - ~100 lines

Result: 2 files x 120-280 lines.

### 2e. `connection/connection_io.rs` (368 lines, PROD=368)

0 structs, 3 free functions (release_pending, nonblock_read, nonblock_write). Already close to 350. Only 18 lines over the limit.

Minimal benefit from splitting. Options:

```
Option A: Keep as-is (368 is close to 350, acceptable)
Option B: Split into io + dispatch:
  connection_io.rs               (~250 lines) -> nonblock_read + nonblock_write
  connection_dispatch.rs         (~120 lines) -> release_pending + spawn_connection_loop
```

Result: Optional. The file is small enough that splitting adds complexity without clear benefit.

---

## ESTIMATED IMPACT

Before: 10,996 lines across 37 files. 8 production files >350 lines.
After:  10,996 lines across ~50 files. Zero files >350 lines.

### Final file count by module

```
codec/
  reader.rs              (342 lines)        OK
  reader_tests.rs        (478 lines)        EXTRACTED
  writer.rs              (230 lines)        OK
  roundtrip.rs           (23 lines)         TINY
  roundtrip_tests.rs     (487 lines)        EXTRACTED

client/
  mod.rs                 (~46 lines)        OK
  client.rs              (~220 lines)       SPLIT
  client_timeout.rs      (~140 lines)       SPLIT
  client_url.rs          (~100 lines)       SPLIT
  client_commands.rs     (~40 lines)        SPLIT
  client_tests.rs        (~180 lines)       EXTRACTED
  client_integration_tests.rs (~420 lines)  EXTRACTED
  pipeline.rs            (318 lines)        OK
  in_memory.rs           (344 lines)        OK
  in_memory_tests.rs     (410 lines)        EXTRACTED

protocol/
  mod.rs                 (~32 lines)        OK
  builder.rs             (293 lines)        OK
  builder_tests.rs       (271 lines)        EXTRACTED
  commands/mod.rs        (54 lines)         OK
  commands/strings.rs    (~217 lines)
  commands/hashes.rs     (~127 lines)
  commands/sets.rs       (~130 lines)
  commands/lists.rs      (~130 lines)
  commands/sorted_sets.rs (~216 lines)
  commands/pubsub.rs     (~95 lines)
  commands/transactions.rs (~50 lines)
  commands/admin.rs      (~230 lines)
  commands_tests.rs      -> SPLIT into 8 files
  fake.rs                (235 lines)        OK
  fake_tests.rs          (119 lines)        OK

core/
  mod.rs                 (~42 lines)        OK
  value.rs               (107 lines)        OK
  error.rs               (138 lines)        OK
  error_tests.rs         (220 lines)        OK
  to_args.rs             (327 lines)        OK
  from_value.rs          (160 lines)        OK
  from_value_tests.rs    (439 lines)        EXTRACTED

tls/
  mod.rs                 (~20 lines)        NEW
  config.rs              (~200 lines)       NEW
  connector.rs           (~220 lines)       NEW
  tests.rs               (~56 lines)        EXTRACTED

connection/
  mod.rs                 (~61 lines)        OK
  connection.rs          (~280 lines)       SPLIT
  connection_limits.rs   (~120 lines)       NEW
  connection_io.rs       (~250 lines)       SPLIT (optional)
  connection_dispatch.rs (~120 lines)       NEW (optional)
  tcp.rs                 (~277 lines)       OK after test extraction
  tcp_tests.rs           (~100 lines)       EXTRACTED
  connection_tests.rs    (489 lines)        KEEP (test module only)
  test_limits.rs         (134 lines)        OK

lib.rs                   (35 lines)         OK
```

### Biggest wins (by lines removed from single file)

1. codec/roundtrip.rs: 510 -> 23 + 487 (production goes 510 -> 23)
2. protocol/commands_tests.rs: 875 -> 8 files x ~100 lines
3. client/client.rs: 557 -> 4 files (220 + 140 + 100 + 40)
4. tls/mod.rs: 552 -> 4 files (20 + 200 + 220 + 56)
5. core/from_value.rs: 599 -> 160 + 439

---

## Execution order (suggested)

1. Priority 0: Extract test-only files (commands_tests, client_tests)
2. Priority 1: Extract embedded tests (roundtrip, from_value, reader, in_memory, builder)
3. Priority 2: Split remaining production files (client.rs, tls, tcp, connection, connection_io)

Each step verified with `cargo check --lib` and `cargo test --lib`.
