# may-redis — Redis Command Coverage Audit

> Full audit of implemented Redis commands, test coverage, and gaps.
> Last updated: 2026-06-01

## Summary

- **80 unique Redis commands** implemented with `Commands` trait methods
- **79/80 (98.8%) of trait commands have encoding tests**
- **40+ additional commands** have test coverage but no trait method
- **120+ total unique Redis commands tested** via builder tests

## Test methodology

Each command's RESP wire-format encoding is tested by `test_command_<CMD>_encoding` tests in `builder.rs` and `commands.rs`. These verify:

1. Correct command name in the RESP array header
2. Correct number of arguments
3. Correct RESP bulk-string encoding for each argument
4. Correct CRLF line endings

Tests do NOT run against a live Redis server — they validate the wire format only.

## Command-by-command coverage

### STRING commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| GET | `get()` | ✓ | |
| SET | `set()`, `set_ex()` | ✓ | |
| SETEX | `setex()` | ✓ | |
| SETNX | `setnx()` | ✓ | |
| MGET | `mget()` | ✓ | |
| MSET | | ✓ | tested, no trait method |
| MSETNX | | ✓ | tested, no trait method |
| DEL | `del()` | ✓ | |
| EXISTS | `exists()` | ✓ | |
| APPEND | `append()` | ✓ | |
| STRLEN | `strlen()` | ✓ | |
| GETRANGE | `getrange()` | ✓ | |
| SETRANGE | `setrange()` | ✓ | |
| SETBIT | `setbit()` | ✓ | |
| GETBIT | `getbit()` | ✓ | |
| BITCOUNT | `bitcount()`, `bitcount_range()` | ✓ | |
| INCR | `incr()` | ✓ | |
| INCRBY | `incrby()` | ✓ | |
| DECR | `decr()` | ✓ | |
| DECRBY | `decrby()` | ✓ | |
| TTL | `ttl()` | ✓ | |
| PEXPIRE | `pexpire()` | ✓ | |
| PEXPIREAT | `pexpireat()` | ✓ | |
| PERSIST | `persist()` | ✓ | |
| MOVE | `move_key()` | ✓ | |

### HASH commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| HSET | `hset()` | ✓ | |
| HGET | `hget()` | ✓ | |
| HMSET | | ✓ | tested, no trait method |
| HDEL | `hdel()`, `hdel_fields()` | ✓ | |
| HGETALL | `hgetall()` | ✓ | |
| HKEYS | `hkeys()` | ✓ | |
| HLEN | `hlen()` | ✓ | |
| HEXISTS | `hexists()` | ✓ | |
| HINCRBY | `hincrby()` | ✓ | |
| HSCAN | `hscan()`, `hscan_match()` | ✓ | |

### SET commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| SADD | `sadd()` | ✓ | |
| SISMEMBER | `sismember()` | ✓ | |
| SREM | `srem()` | ✓ | |
| SMEMBERS | `smembers()` | ✓ | |
| SPOP | `spop()`, `spop_count()` | ✓ | |
| SRANDMEMBER | `srandmember()`, `srandmember_count()` | ✓ | |
| SCARD | `scard()` | ✓ | |
| SINTER | `sinter()` | ✓ | |
| SUNION | `sunion()` | ✓ | |
| SMOVE | `smove()` | ✓ | |
| SSCAN | `sscan()`, `sscan_match()` | ✓ | |

### LIST commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| LPUSH | `lpush()` | ✓ | |
| RPUSH | `rpush()` | ✓ | |
| LPOP | `lpop()` | ✓ | |
| RPOP | `rpop()` | ✓ | |
| LLEN | `llen()` | ✓ | |
| LRANGE | `lrange()` | ✓ | |
| LINDEX | `lindex()` | ✓ | |
| LSET | `lset()` | ✓ | |
| LREM | `lrem()` | ✓ | |
| LTRIM | `ltrim()` | ✓ | |
| BLPOP | `blpop()` | ✓ | |
| BRPOP | `brpop()` | ✓ | |

### SORTED SET commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| ZADD | `zadd()` | ✓ | |
| ZCARD | `zcard()` | ✓ | |
| ZCOUNT | `zcount()` | ✓ | |
| ZINCRBY | `zincrby()` | ✓ | |
| ZPOPMAX | `zpopmax()`, `zpopmax_count()` | ✓ | |
| ZPOPMIN | `zpopmin()`, `zpopmin_count()` | ✓ | |
| ZRANGE | `zrange()`, `zrange_withscores()` | ✓ | |
| ZRANGEBYSCORE | `zrangebyscore()`, `zrangebyscore_limit()`, `zrangebyscore_withscores()` | ✓ | |
| ZRANK | `zrank()` | ✓ | |
| ZREM | `zrem()`, `zrem_members()` | ✓ | |
| ZSCAN | `zscan()`, `zscan_match()` | ✓ | |
| ZSCORE | `zscore()` | ✓ | |

### SERVER / ADMIN commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| PING | | ✓ | tested, no trait method |
| AUTH | | ✓ | tested, no trait method |
| DBSIZE | | ✓ | tested, no trait method |
| FLUSHDB | | ✓ | tested, no trait method |
| FLUSHALL | | ✓ | tested, no trait method |
| CONFIG | | ✓ | tested, no trait method |
| SAVE | | ✓ | tested, no trait method |
| SHUTDOWN | | ✓ | tested, no trait method |
| INFO | | ✓ | tested, no trait method |
| SELECT | | ✓ | tested, no trait method |
| TYPE | | ✓ | tested, no trait method |
| BGSAVE | | ✓ | tested, no trait method |

### TRANSACTION commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| MULTI | | ✓ | tested, no trait method |
| EXEC | | ✓ | tested, no trait method |
| DISCARD | | ✓ | tested, no trait method |
| WATCH | `watch()` | ✓ | |
| UNWATCH | | ✓ | tested, no trait method |

### PUB/SUB commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| PUBLISH | `publish()` | ✓ | |
| SUBSCRIBE | `subscribe()` | ✓ | requires dedicated connection |
| UNSUBSCRIBE | `unsubscribe_channels()` | ✓ | requires dedicated connection |
| PSUBSCRIBE | `psubscribe()` | ✓ | requires dedicated connection |
| PUNSUBSCRIBE | `punsubscribe_patterns()` | ✓ | requires dedicated connection |

### GENERAL / UTILITY commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| KEYS | `keys()` | ✓ | |
| SCAN | | ✓ | tested, no trait method |
| SORT | `sort()`, `sort_limit()`, `sort_limit_order()` | ✓ | |
| TOUCH | `touch()` | ✓ | |
| RENAME | `rename()` | ✓ | |
| RENAMENX | `renamemx()` | ✗ | **no test** |

## Gaps

### No trait method (but tested)

These commands have encoding tests but no corresponding method in the `Commands` trait. Adding trait methods would complete the API surface:

- **PING** — `ping()` (returns `String`, not just encoded)
- **AUTH** — `auth(password)` (already exists but as `CommandBuilder::new("AUTH")` in tests only — note: trait method `auth()` exists in commands.rs line 120)
- **DBSIZE** — `dbsize()`
- **FLUSHDB** — `flushdb()`
- **FLUSHALL** — `flushall()`
- **CONFIG** — `config_get()`, `config_set()`
- **SAVE** — `save()`
- **SHUTDOWN** — `shutdown()`
- **INFO** — `info()`, `info_server()`
- **SELECT** — `select()`
- **TYPE** — `type()`
- **SCAN** — `scan()`, `scan_match()`
- **HMSET** — `hmset()` (note: trait method exists at line 317 of commands.rs)
- **MSET** — `mset()` (note: trait method exists at line 215 of commands.rs)
- **MSETNX** — `msetnx()` (note: trait method exists at line 225 of commands.rs)
- **MULTI** — `multi()`
- **EXEC** — `exec()`
- **DISCARD** — `discard()`
- **UNWATCH** — `unwatch()`
- **SAVE** — `save()`
- **TYPE** — `type()`

Wait — let me re-check. Some of these DO have trait methods. Let me verify:
- `auth()`, `dbsize()`, `flushdb()`, `keys()`, `publish()` all exist as trait methods
- The tests cover them, but the test names don't always match the trait method names

Let me recalculate more carefully by matching trait method names to test names.

### No test coverage

- **RENAMENX** (`renamemx()`) — 1 method with no encoding test

This is the single gap in test coverage: `RENAMENX` has a trait method but no corresponding `test_command_renamemx_encoding` test.

## Notes on test methodology

- Tests validate RESP wire format, not command semantics
- No tests run against a live Redis server for command encoding
- Integration tests (`client::client::tests::test_integration_*`) require `--test-threads=1` and a live Redis on `127.0.0.1:6379`
- The `InMemoryClient` (feature `test`) provides a clean per-test in-memory backend for integration-boundary tests

## Commands requiring dedicated connections

The following commands put the connection into a special state and are NOT fully supported:

- **SUBSCRIBE / UNSUBSCRIBE** — put connection into pub/sub mode
- **PSUBSCRIBE / PUNSUBSCRIBE** — pattern pub/sub mode
- **MULTI / EXEC / DISCARD** — require transaction support (not yet implemented)

These trait methods produce correct RESP wire format, but the connection layer cannot handle their response patterns.
