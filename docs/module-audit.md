# Module Breakdown Audit

**Date:** 2026-05-29
**Scope:** All files under `src/`
**Total lines:** 10,885 across 23 files

## Clippy Config Change

`too_many_lines` changed from `allow` to `warn` in `Cargo.toml`.

> **Note:** clippy's `too_many_lines` lint fires per-**function**, not per-**file**. There is no built-in clippy lint for file-level line count limits. To enforce a 350-line-per-file rule you need a custom clippy lint or a pre-commit hook. The `too_many_lines = "warn"` will at least flag individual functions exceeding 350 lines.

## Severity Key

| Level | Lines | Action |
|-------|-------|--------|
| OK | < 100 | No action needed |
| LOW | 100-299 | Extract tests to separate file |
| MEDIUM | 300-499 | Extract tests + consider further split |
| HIGH | 500-999 | Extract tests + split production code |
| CRITICAL | 1000+ | Major structural reorganization |

---

## Current vs. Target File Map

### CRITICAL — Immediate action required

| Current File | Lines | Target File | Lines | Rationale |
|---|---|---|---|---|
| `protocol/commands.rs` | 1,988 | `protocol/commands/mod.rs` | ~80 | Trait definition + module re-exports |
| | | `protocol/commands/strings.rs` | ~220 | GET, SET, SETEX, SETNX, MGET, MSET, MSETNX, APPEND, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, DECR, DECRBY, INCR, INCRBY |
| | | `protocol/commands/strings_admin.rs` | ~100 | KEYS, KEYS*, DBSIZE, FLUSHDB, FLUSHALL, SELECT, TYPE, MOVE, RENAME, RENAMENX, SORT, SCAN, TOUCH, SAVE, BGSAVE, SHUTDOWN |
| | | `protocol/commands/hashes.rs` | ~200 | HSET, HGET, HDEL, HGETALL, HKEYS, HMSET, HINCRBY, HLEN, HEXISTS, HSCAN, HSCAN MATCH |
| | | `protocol/commands/sets.rs` | ~150 | SADD, SISMEMBER, SREM, SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION, SMOVE, SSCAN |
| | | `protocol/commands/lists.rs` | ~150 | LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP |
| | | `protocol/commands/sorted_sets.rs` | ~250 | ZADD, ZADD_MULTI, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZSCAN, ZRANGEBYSCORE variants |
| | | `protocol/commands/pubsub.rs` | ~100 | SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE, PUBLISH |
| | | `protocol/commands/transactions.rs` | ~60 | MULTI, EXEC, DISCARD, WATCH, UNWATCH |
| | | `protocol/commands/admin.rs` | ~100 | PING, AUTH, FLUSHDB, PEXPIRE, PEXPIREAT, PERSIST, PTTL, TTL, EXPIRE |
| **Subtotal** | **1,988** | | **~1,560** | Split by Redis data domain |
| | | *(tests moved to target files)* | | |
| `connection/connection.rs` | 1,238 | `connection/connection.rs` | ~350 | Structs, process_req, nonblock_read/write, decode_responses, spawn_connection_loop, Connection impl, Drop |
| | | `connection/tests.rs` | ~600 | All unit tests, `#[cfg(test)]` |
| **Subtotal** | **1,238** | | **~950** | Extract tests out |
| `client/client.rs` | 1,217 | `client/client.rs` | ~120 | URL decoding, scheme, TimeoutGuard, RedisClient struct, connect, execute, ping, pipeline |
| | | `client/commands_impl.rs` | ~80 | `impl Commands for RedisClient` |
| | | `client/tests.rs` | ~500 | All integration tests, `#[cfg(test)]` |
| **Subtotal** | **1,217** | | **~700** | Extract tests + move Commands impl |

### HIGH — Extract tests, moderate production split

| Current File | Lines | Target File | Lines | Rationale |
|---|---|---|---|---|
| `codec/reader.rs` | 820 | `codec/reader.rs` | ~340 | DepthGuard + RESPReader impl |
| | | `codec/reader_tests.rs` | ~480 | All tests, `#[cfg(test)]` |
| **Subtotal** | **820** | | **~820** | Extract tests out |
| `client/in_memory.rs` | 754 | `client/in_memory.rs` | ~350 | InMemoryStore, InMemoryClient, glob_match |
| | | `client/in_memory_tests.rs` | ~400 | All tests, `#[cfg(test)]` |
| **Subtotal** | **754** | | **~750** | Extract tests out |
| `tls/mod.rs` | 552 | `tls/config.rs` | ~130 | TlsVersion, RustlsRootCerts, ClientCerts, TlsConfig |
| | | `tls/connector.rs` | ~180 | SkipVerifier, TlsConnector |
| | | `tls/stream.rs` | ~100 | TlsStream + Read/Write impls |
| | | `tls/error.rs` | ~35 | TlsError + Display + Error impls |
| | | `tls/tests.rs` | ~35 | Tests, `#[cfg(test)]` |
| | | `tls/mod.rs` | ~50 | Module re-exports |
| **Subtotal** | **552** | | **~530** | Split into natural submodules |
| `core/from_value.rs` | 599 | `core/from_value.rs` | ~300 | All FromRedisValue impls |
| | | `core/from_value_tests.rs` | ~300 | All tests, `#[cfg(test)]` |
| **Subtotal** | **599** | | **~600** | Extract tests out |

### MEDIUM — Extract tests, small production files

| Current File | Lines | Target File | Lines | Rationale |
|---|---|---|---|---|
| `protocol/fake.rs` | 354 | `protocol/fake.rs` | ~180 | FakeConnection struct + impl |
| | | `protocol/fake_tests.rs` | ~174 | All tests, `#[cfg(test)]` |
| **Subtotal** | **354** | | **~354** | Extract tests out |
| `core/error.rs` | 360 | `core/error.rs` | ~180 | RedisError, RedisResult, Display, Error impls |
| | | `core/error_tests.rs` | ~180 | All tests, `#[cfg(test)]` |
| **Subtotal** | **360** | | **~360** | Extract tests out |
| `codec/roundtrip.rs` | 510 | `codec/roundtrip_tests.rs` | ~510 | All roundtrip tests (production is trivial `roundtrip()` helper, ~15 lines) |
| **Subtotal** | **510** | | **~510** | Extract tests out |
| `connection/tcp.rs` | 477 | `connection/tcp.rs` | ~300 | TcpConnector, resolve, connect_addr_with_timeout, ssrf_allowed, SsrfConfig |
| | | `connection/tcp_tests.rs` | ~177 | All tests, `#[cfg(test)]` |
| **Subtotal** | **477** | | **~477** | Extract tests out |

### LOW / OK — No structural changes needed

| Current File | Lines | Status | Action |
|---|---|---|---|
| `codec/writer.rs` | 230 | OK | No change |
| `core/value.rs` | 180 | OK | No change |
| `core/to_args.rs` | 327 | LOW | Extract tests (optional) |
| `client/pipeline.rs` | 318 | LOW | Extract tests (optional) |
| `connection/test_limits.rs` | 134 | OK | No change |
| `connection/mod.rs` | 65 | OK | No change |
| `client/mod.rs` | 50 | OK | No change |
| `codec/mod.rs` | 39 | OK | No change |
| `protocol/mod.rs` | 32 | OK | No change |
| `core/mod.rs` | 42 | OK | No change |
| `lib.rs` | 35 | OK | No change |

---

## Target Directory Structure

```
src/
  lib.rs                              35   OK
  core/
    mod.rs                            42   OK
    value.rs                         180   OK
    error.rs                         180   (extracted tests out)
    error_tests.rs                   180   #[cfg(test)]
    from_value.rs                    300   (extracted tests out)
    from_value_tests.rs              300   #[cfg(test)]
    to_args.rs                       327   OK

  codec/
    mod.rs                            39   OK
    writer.rs                        230   OK
    reader.rs                        340   (extracted tests out)
    reader_tests.rs                  480   #[cfg(test)]

  protocol/
    mod.rs                            32   OK
    fake.rs                          180   (extracted tests out)
    fake_tests.rs                    174   #[cfg(test)]
    builder.rs                       564   MEDIUM — no split planned (small functions)
    commands/                        (NEW directory)
      mod.rs                          80   Trait definition + re-exports
      strings.rs                    220   String commands
      strings_admin.rs              100   String admin commands
      hashes.rs                     200   Hash commands
      sets.rs                       150   Set commands
      lists.rs                      150   List commands
      sorted_sets.rs                250   Sorted set commands
      pubsub.rs                     100   Pub/Sub commands
      transactions.rs                60   Transaction commands
      admin.rs                     100   General admin commands

  connection/
    mod.rs                            65   OK
    tcp.rs                           300   (extracted tests out)
    tcp_tests.rs                     177   #[cfg(test)]
    connection.rs                    350   (extracted tests out)
    connection_tests.rs              600   #[cfg(test)]

  client/
    mod.rs                            50   OK
    client.rs                        120   (simplified + tests out)
    commands_impl.rs                  80   impl Commands for RedisClient
    pipeline.rs                      318   OK
    in_memory.rs                     350   (extracted tests out)
    in_memory_tests.rs               400   #[cfg(test)]
    tests.rs                         500   #[cfg(test)]

  tls/
    mod.rs                            50   Re-exports only
    config.rs                        130   TlsVersion, RustlsRootCerts, ClientCerts, TlsConfig
    connector.rs                     180   SkipVerifier, TlsConnector
    stream.rs                        100   TlsStream + Read/Write impls
    error.rs                          35   TlsError
    tests.rs                          35   #[cfg(test)]
```

---

## Summary Statistics

| Category | Before | After |
|----------|--------|-------|
| Total files | 23 | ~36 |
| Production files | 18 | ~24 |
| Test files (extracted) | 5 | ~12 |
| New directories | 0 | 2 (commands/, tls/) |
| Max single file (production) | 1,988 | ~564 (protocol/builder.rs) |
| Production files > 350 lines | 8 | 1 (protocol/builder.rs — small functions, no natural split point) |
| Production files < 350 lines | 10 | ~23 |
| Average production file size | 605 lines | ~260 lines |

**Note:** `protocol/builder.rs` (564 lines) remains above 350 but its functions are all small (avg ~20 lines) and there is no natural semantic split point. It could be split later if needed.

---

## Implementation Order (Recommended)

### Phase 1 — Biggest impact, safest (extract tests first)

1. `connection/connection.rs` -> extract tests to `connection_tests.rs`
2. `client/client.rs` -> extract tests to `tests.rs`, move Commands impl to `commands_impl.rs`
3. `protocol/commands.rs` -> split into domain sub-modules (new directory)
4. `codec/reader.rs` -> extract tests to `reader_tests.rs`
5. `client/in_memory.rs` -> extract tests to `in_memory_tests.rs`

### Phase 2 — Polish

6. `tls/mod.rs` -> split into config.rs, connector.rs, stream.rs, error.rs
7. `core/from_value.rs` -> extract tests
8. `core/error.rs` -> extract tests
9. `codec/roundtrip.rs` -> extract tests
10. `connection/tcp.rs` -> extract tests
11. `protocol/fake.rs` -> extract tests

### Phase 3 — Optional

12. `core/to_args.rs` -> extract tests
13. `client/pipeline.rs` -> extract tests
14. `protocol/builder.rs` -> split if needed (small functions, hard to justify)

---

## Splitting Strategy Details

### Pattern: test extraction

For each file, keep production code in `<module>.rs` and move all `#[cfg(test)] mod tests { ... }` blocks into a new `<module>_tests.rs` file.

In the parent `mod.rs` or the module file, add:

```rust
#[cfg(test)]
mod tests; // or #[path = "module_tests.rs"] mod tests;
```

The test file uses `use super::*;` to access the module's items.

### Pattern: commands module split

The `Commands` trait lives in `protocol/commands/mod.rs`. Each domain file (e.g. `strings.rs`) defines its own trait:

```rust
pub trait StringCommands {
    fn get<K: ToRedisArgs>(&self, key: K) -> CommandBuilder { ... }
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder { ... }
    // ...
}
```

Then the main trait extends all domain traits:

```rust
pub trait Commands: StringCommands + HashCommands + SetCommands + ... {}
```

The blanket impl for `()` is:

```rust
impl<T: StringCommands + HashCommands + ...> Commands for T {}
```

This avoids duplicating all 167 methods in `impl Commands for RedisClient`.

### Pattern: TLS submodule split

Each sub-module declares its items publicly. The parent `mod.rs` re-exports:

```rust
mod config;
mod connector;
mod stream;
mod error;

pub use config::{TlsConfig, TlsVersion, RustlsRootCerts, ClientCerts};
pub use connector::TlsConnector;
pub use stream::TlsStream;
pub use error::TlsError;
```

---

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | `too_many_lines = "warn"` (clippy lint) |
| `src/protocol/commands.rs` | DELETED — replaced by sub-modules |
| `src/protocol/commands/*.rs` | CREATED — domain-specific command impls |
| `src/connection/connection.rs` | REDUCED — tests extracted |
| `src/connection/connection_tests.rs` | CREATED |
| `src/connection/tcp.rs` | REDUCED — tests extracted |
| `src/connection/tcp_tests.rs` | CREATED |
| `src/client/client.rs` | REDUCED — tests + Commands impl extracted |
| `src/client/commands_impl.rs` | CREATED |
| `src/client/tests.rs` | CREATED |
| `src/codec/reader.rs` | REDUCED — tests extracted |
| `src/codec/reader_tests.rs` | CREATED |
| `src/client/in_memory.rs` | REDUCED — tests extracted |
| `src/client/in_memory_tests.rs` | CREATED |
| `src/tls/mod.rs` | SIMPLIFIED — re-exports only |
| `src/tls/config.rs` | CREATED |
| `src/tls/connector.rs` | CREATED |
| `src/tls/stream.rs` | CREATED |
| `src/tls/error.rs` | CREATED |
| `src/tls/tests.rs` | CREATED |
| `src/core/from_value.rs` | REDUCED — tests extracted |
| `src/core/from_value_tests.rs` | CREATED |
| `src/core/error.rs` | REDUCED — tests extracted |
| `src/core/error_tests.rs` | CREATED |
| `src/codec/roundtrip.rs` -> `src/codec/roundtrip_tests.rs` | RENAMED |
| `src/protocol/fake.rs` | REDUCED — tests extracted |
| `src/protocol/fake_tests.rs` | CREATED |
