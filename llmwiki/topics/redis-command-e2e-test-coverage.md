---
title: Redis Command E2E Test Coverage
created: 2026-06-01
updated: 2026-06-01
type: concept
tags: [testing, coverage, redis, library]
sources: []
confidence: high
---

# Redis Command E2E Test Coverage

> Comprehensive audit of may-redis test coverage across all 122 command methods.

## Current State

### Command Surface
- **122 command methods** defined across 8 traits:
  - StringsCommands (24), HashesCommands (12), ListsCommands (12), SetsCommands (14),
  - SortedSetsCommands (20), AdminCommands (28), PubsubCommands (7), TransactionsCommands (5)

### Coverage Metrics

| Metric | Value |
|--------|-------|
| Total command methods | 122 |
| Protocol encoding tests (unit) | 122 (100%) |
| Real-data integration tests | 87 (covering ~37 distinct commands) |
| E2E perf tests | ~7 distinct command families |
| Commands with ZERO real-data testing | 85 (70%) |
| Traits with ZERO integration tests | 2/8 (Transactions, Admin partial) |

### Phase 1 (P0) — COMPLETED

Hash (14 tests), Lists (11 tests), Sets (11 tests), SortedSets (12 tests) — 48 integration tests added.

Coverage progression: ~12% → ~22% real-data integration coverage.

### Phase 2 (P1) — COMPLETED

Admin (21 tests), Transactions (5 tests) — 26 integration tests added.

Coverage progression: ~22% → ~35% real-data integration coverage.

### What IS tested end-to-end
- Basic key-value: SET, GET, DEL, EXISTS
- Atomic increment: INCR
- TTL management: SET EX, EXPIRE, TTL
- Key matching: KEYS
- Connection: PING, CLONE
- Batching: PIPELINE
- Error paths: type mismatches, null responses, server errors
- Concurrency: multiple coroutines, concurrent pipelines

### What is MISSING end-to-end
- All 12 Hash commands (HSET, HGET, HDEL, HKEYS, HGETALL, HLEN, HEXISTS, HSCAN, HINCRBY, HMSET, HVALS, HSCAN_MATCH)
- All 12 List commands (LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP)
- All 14 Set commands (SADD, SISMEMBER, SREM, SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION, SSCAN, SMOVE, etc.)
- All 20 SortedSet commands (ZADD, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZSCAN, etc.)
- Multi-key string ops: MGET, MSET, MSETNX, SETNX, SETEX
- String bit/range ops: SETBIT, GETBIT, BITCOUNT, GETRANGE, SETRANGE, APPEND, STRLEN
- Increment/decrement: INCRBY, DECR, DECRBY
- Transaction primitives: MULTI, EXEC, DISCARD, WATCH, UNWATCH
- Key management: TYPE, MOVE, RENAME, RENAMENX
- Scanning: SCAN, HSCAN, SSCAN, ZSCAN (only encoding tested)
- Admin/monitoring: CONFIG, INFO, SAVE, BGSAVE, PTTL, PEXPIRE, PEXPIREAT, PERSIST, TOUCH

### In-memory backend gaps
Current `InMemoryClient` (feature `test`) only supports: SET, GET, DEL, EXISTS, INCR, TTL, EXPIRE, KEYS, DBSIZE, FLUSHDB.
Missing: Hash/List/Set/SortedSet operations entirely.

## Test Architecture

Tests live in `src/client/client_tests/` directory following established pattern:
- Each integration test calls `FLUSHDB` before/after for isolation
- All tests use `may::run` + `may::go!` pattern (no tokio)
- Tests verified with live Redis via GitHub Actions service container (`redis:7-alpine`)

## Implementation Plan (PRD)

See [[docs/PRD-e2e-test-coverage]] for full 6-phase implementation plan:

1. **Phase 1 (P0):** Hash (14 tests), Lists (11 tests), Sets (15 tests), SortedSets (22 tests) — ~62 tests against live Redis
2. **Phase 2 (P1):** Admin (19 tests), Transactions (6 tests)
3. **Phase 3 (P2):** Strings extension (20 tests)
4. **Phase 4 (P2):** Expanded concurrency tests for all command families
5. **Phase 5 (P2):** SCAN family + error-path tests
6. **Phase 6 (P3):** In-memory backend extension + in-memory tests

## Acceptance Criteria

- All 122 command methods have at least one integration test
- 319 existing tests all pass (no regressions)
- All new test files under 350 lines
- `cargo clippy --lib --tests --all-features -- -D warnings` clean
- `cargo fmt --all --check` clean
- In-memory backend supports Hash/List/Set/SortedSet minimum

## Related

- [[may-coroutine-pattern]] — May coroutine test infrastructure
- [[resp-protocol]] — RESP2 wire format (encoding tests cover 100%)
- [[sesame-idam-integration]] — Sesame-IDAM Redis usage (11 canonical commands, 100% coverage)
- [[may-redis-epic-7-story-1]] — String Extension commands (13 added, 34 total tests)
- [[command-mapping]] — Commands trait method to RESP wire format mapping reference
- [[module-breakdown]] — Test extraction from monolithic files into sub-modules
