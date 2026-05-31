---
title: Redis Command Coverage
created: 2026-06-01
updated: 2026-06-01
type: comparison
tags: [testing, coverage, redis]
sources: []
---

# Redis Command Coverage

> 3-layer audit: Redis canonical commands vs may-redis implementation vs sesame-idam usage.

## Layer 1: Redis Canonical

Redis has ~200 commands organized by data type. may-redis v1 targets RESP2 wire format with a focused subset.

## Layer 2: may-redis Implementation

### Implemented (122 command methods across 8 traits)

| Trait | Methods | Status |
|-------|---------|--------|
| StringsCommands | 24 | ✅ All 24 methods |
| HashesCommands | 12 | ✅ All 12 methods |
| ListsCommands | 12 | ✅ All 12 methods |
| SetsCommands | 14 | ✅ All 14 methods |
| SortedSetsCommands | 20 | ✅ All 20 methods |
| AdminCommands | 28 | ✅ All 28 methods |
| PubsubCommands | 7 | ✅ All 7 methods |
| TransactionsCommands | 5 | ✅ All 5 methods |

### Protocol Encoding Tests
- 100% coverage — every command has RESP2 wire-format tests
- Tests verify `cmd().arg().build()` produces correct RESP bytes

### Integration Tests (Real Redis) — Updated after Phase 1 (P0)
- ~22% coverage — Hash, Lists, Sets, SortedSets added
- Integration tests in `src/client/client_tests/` directory
- Uses `may::run` + `may::go!` pattern with `redis:7-alpine` container

#### Phase 1 (P0) — 48 tests added
- **Hash (28 tests):** HSET, HGET, HGETALL, HDEL (single/multi), HKEYS, HVALS, HLEN, HEXISTS, HINCRBY, HSCAN, HSCAN_MATCH, empty key, 1000-field, type mismatch, pipeline, concurrent
- **Lists (11 tests):** LPUSH, RPUSH, LPOP, RPOP, LRANGE, LINDEX, LLEN, LSET, LREM, LTRIM, empty key, concurrent
- **Sets (11 tests):** SADD, SMEMBERS, SISMEMBER, SREM, SCARD, SPOP, SRANDMEMBER, SINTER, SUNION, SSCAN, empty key
- **SortedSets (12 tests):** ZADD, ZREM, ZCARD, ZRANK, ZSCORE, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZRANGE, ZRANGEBYSCORE, ZSCAN, empty key

#### Still missing real-data tests
- **AdminCommands:** TYPE, MOVE, RENAME, RENAMENX, TOUCH, PTTL, PEXPIRE, PEXPIREAT, PERSIST, SELECT, SORT, SCAN, SAVE, BGSAVE, FLUSHALL, SHUTDOWN, INFO, CONFIG
- **TransactionsCommands:** MULTI, EXEC, DISCARD, WATCH, UNWATCH
- **Multi-key strings:** MGET, MSET, MSETNX, SETNX, SETEX, SETBIT, GETBIT, BITCOUNT, GETRANGE, SETRANGE, APPEND, STRLEN, INCRBY, DECR, DECRBY

## Layer 3: Sesame-IDAM Usage

Sesame-IDAM uses 11 canonical Redis commands across 5 modules:
- `GET`, `SET`, `DEL`, `TTL` — key-value operations
- `EXISTS`, `EXPIRE` — key management
- `INCR` — atomic counters (token versioning)
- `PUBLISH` — pub/sub (token version notifications)
- `KEYS`, `DBSIZE`, `FLUSHDB` — monitoring/cleanup (tests only)

**Result: 100% sesame-idam command coverage** — all 11 canonical commands implemented.

## Gap Analysis

The ~85 methods not tested against live Redis represent:
1. **Admin/monitoring** — TYPE, MOVE, RENAME, RENAMENX, TOUCH, PTTL, PEXPIRE, PEXPIREAT, PERSIST, SELECT, SORT, SCAN, SAVE, BGSAVE, FLUSHALL, SHUTDOWN, INFO, CONFIG (18 commands)
2. **Transactions** — MULTI, EXEC, DISCARD, WATCH, UNWATCH (5 commands)
3. **Multi-key string ops** — MGET, MSET, MSETNX, SETNX, SETEX, SETBIT, GETBIT, BITCOUNT, GETRANGE, SETRANGE, APPEND, STRLEN, INCRBY, DECR, DECRBY (15 commands)

## Test Strategy

Three-layer testing:
1. **Unit tests** — pure encoding/decoding, no runtime needed
2. **FakeConnection tests** — protocol layer, no network needed
3. **Integration tests** — live Redis, `may::run` + `may::go!`

See [[redis-command-e2e-test-coverage]] for full audit and implementation plan.
