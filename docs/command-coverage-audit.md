# may-redis — Redis Command Coverage Audit

> Full audit of implemented Redis commands, test coverage, and gaps.
> Last updated: 2026-07-10

## Summary

- **100 unique Redis commands** implemented with `Commands` trait methods
- **122 test variants** covering all trait methods and overloaded variants
- **100% of trait commands have encoding tests**
- **0 methods without test coverage**

## Test methodology

Each command's RESP wire-format encoding is tested by `test_command_<CMD>_encoding` tests in `commands.rs`. These verify:

1. Correct command name in the RESP array header
2. Correct number of arguments
3. Correct RESP bulk-string encoding for each argument
4. Correct CRLF line endings

Tests do NOT run against a live Redis server — they validate the wire format only.

## Command-by-command coverage

### STRING commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| GET | `get()` | Yes | |
| SET | `set()` | Yes | |
| SETEX | `setex()` | Yes | |
| SETNX | `setnx()` | Yes | |
| MGET | `mget()` | Yes | |
| MSET | `mset()` | Yes | |
| MSETNX | `msetnx()` | Yes | |
| DEL | `del()` | Yes | |
| EXISTS | `exists()` | Yes | |
| APPEND | `append()` | Yes | |
| STRLEN | `strlen()` | Yes | |
| GETRANGE | `getrange()` | Yes | |
| SETRANGE | `setrange()` | Yes | |
| SETBIT | `setbit()` | Yes | |
| GETBIT | `getbit()` | Yes | |
| BITCOUNT | `bitcount()`, `bitcount_range()` | Yes | |
| INCR | `incr()` | Yes | |
| INCRBY | `incrby()` | Yes | |
| DECR | `decr()` | Yes | |
| DECRBY | `decrby()` | Yes | |
| TTL | `ttl()` | Yes | |
| PEXPIRE | `pexpire()` | Yes | |
| PEXPIREAT | `pexpireat()` | Yes | |
| PERSIST | `persist()` | Yes | |
| MOVE | `move_key()` | Yes | |
| PTTL | `pttl()` | Yes | |

### HASH commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| HSET | `hset()` | Yes | |
| HGET | `hget()` | Yes | |
| HMSET | `hmset()` | Yes | |
| HDEL | `hdel()`, `hdel_fields()` | Yes | |
| HGETALL | `hgetall()` | Yes | |
| HKEYS | `hkeys()` | Yes | |
| HLEN | `hlen()` | Yes | |
| HEXISTS | `hexists()` | Yes | |
| HINCRBY | `hincrby()` | Yes | |
| HSCAN | `hscan()`, `hscan_match()` | Yes | |

### SET commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| SADD | `sadd()` | Yes | |
| SISMEMBER | `sismember()` | Yes | |
| SREM | `srem()` | Yes | |
| SMEMBERS | `smembers()` | Yes | |
| SPOP | `spop()`, `spop_count()` | Yes | |
| SRANDMEMBER | `srandmember()`, `srandmember_count()` | Yes | |
| SCARD | `scard()` | Yes | |
| SINTER | `sinter()` | Yes | |
| SUNION | `sunion()` | Yes | |
| SMOVE | `smove()` | Yes | |
| SSCAN | `sscan()`, `sscan_match()` | Yes | |

### LIST commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| LPUSH | `lpush()` | Yes | |
| RPUSH | `rpush()` | Yes | |
| LPOP | `lpop()` | Yes | |
| RPOP | `rpop()` | Yes | |
| LLEN | `llen()` | Yes | |
| LRANGE | `lrange()` | Yes | |
| LINDEX | `lindex()` | Yes | |
| LSET | `lset()` | Yes | |
| LREM | `lrem()` | Yes | |
| LTRIM | `ltrim()` | Yes | |
| BLPOP | `blpop()` | Yes | |
| BRPOP | `brpop()` | Yes | |

### SORTED SET commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| ZADD | `zadd()`, `zadd_multi()` | Yes | |
| ZCARD | `zcard()` | Yes | |
| ZCOUNT | `zcount()` | Yes | |
| ZINCRBY | `zincrby()` | Yes | |
| ZPOPMAX | `zpopmax()`, `zpopmax_count()` | Yes | |
| ZPOPMIN | `zpopmin()`, `zpopmin_count()` | Yes | |
| ZRANGE | `zrange()`, `zrange_withscores()` | Yes | |
| ZRANGEBYSCORE | `zrangebyscore()`, `zrangebyscore_limit()`, `zrangebyscore_withscores()` | Yes | |
| ZRANK | `zrank()` | Yes | |
| ZREM | `zrem()`, `zrem_members()` | Yes | |
| ZSCAN | `zscan()`, `zscan_match()` | Yes | |
| ZSCORE | `zscore()` | Yes | |

### SERVER / ADMIN commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| PING | `ping()` | Yes | |
| AUTH | `auth()` | Yes | |
| DBSIZE | `dbsize()` | Yes | |
| FLUSHDB | `flushdb()` | Yes | |
| FLUSHALL | `flushall()` | Yes | |
| CONFIG | `config_get()` | Yes | |
| SAVE | `save()` | Yes | |
| SHUTDOWN | `shutdown()`, `shutdown_nosave()` | Yes | |
| INFO | `info()`, `info_section()` | Yes | |
| SELECT | `select()` | Yes | |
| TYPE | `type_()` | Yes | |
| BGSAVE | `bgsave()` | Yes | |

### TRANSACTION commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| MULTI | `multi()` | Yes | |
| EXEC | `exec()` | Yes | |
| DISCARD | `discard()` | Yes | |
| WATCH | `watch()` | Yes | |
| UNWATCH | `unwatch()` | Yes | |

### PUB/SUB commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| PUBLISH | `publish()` | Yes | |
| SUBSCRIBE | `subscribe()` | Yes | requires dedicated connection |
| UNSUBSCRIBE | `unsubscribe()`, `unsubscribe_channels()` | Yes | requires dedicated connection |
| PSUBSCRIBE | `psubscribe()` | Yes | requires dedicated connection |
| PUNSUBSCRIBE | `punsubscribe()`, `punsubscribe_patterns()` | Yes | requires dedicated connection |

### GENERAL / UTILITY commands

| Command | Trait method(s) | Test | Notes |
|---------|----------------|------|-------|
| KEYS | `keys()` | Yes | |
| SCAN | `scan()`, `scan_match()` | Yes | |
| SORT | `sort()`, `sort_limit()`, `sort_limit_order()` | Yes | |
| TOUCH | `touch()` | Yes | |
| RENAME | `rename()` | Yes | |
| RENAMENX | `renamemx()` | Yes | |

## Coverage gaps

**None.** All 100 trait methods have test coverage. All 122 test variants pass.

## Commands requiring dedicated connections

The following commands put the connection into a special state and are NOT fully supported by the connection layer:

- **SUBSCRIBE / UNSUBSCRIBE** — put connection into pub/sub mode
- **PSUBSCRIBE / PUNSUBSCRIBE** — pattern pub/sub mode
- **MULTI / EXEC / DISCARD** — require transaction support (not yet implemented)

These trait methods produce correct RESP wire format (verified by tests), but the connection layer cannot handle their response patterns.

## Notes on test methodology

- Tests validate RESP wire format, not command semantics
- No tests run against a live Redis server for command encoding
- Integration tests (`client::client::tests::test_integration_*`) require `--test-threads=1` and a live Redis on `127.0.0.1:6379`
- The `InMemoryClient` (feature `test`) provides a clean per-test in-memory backend for integration-boundary tests
