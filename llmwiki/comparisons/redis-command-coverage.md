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

### Integration Tests (Real Redis)
- ~12% coverage — only basic StringsCommands tested against live server
- Integration tests in `src/client/client_tests/integration.rs`
- Uses `may::run` + `may::go!` pattern with `redis:7-alpine` container

### Commands with ZERO integration tests
- All 58 Hash/List/Set/SortedSet methods (5 traits)
- Multi-key string ops, bit/range ops, transaction primitives
- Admin commands: TYPE, MOVE, RENAME, CONFIG, INFO, SAVE, BGSAVE, etc.

## Layer 3: Sesame-IDAM Usage

Sesame-IDAM uses 11 canonical Redis commands across 5 modules:
- `GET`, `SET`, `DEL`, `TTL` — key-value operations
- `EXISTS`, `EXPIRE` — key management
- `INCR` — atomic counters (token versioning)
- `PUBLISH` — pub/sub (token version notifications)
- `KEYS`, `DBSIZE`, `FLUSHDB` — monitoring/cleanup (tests only)

**Result: 100% sesame-idam command coverage** — all 11 canonical commands implemented.

## Gap Analysis

The ~111 methods not tested against live Redis represent:
1. **Data structure operations** — Hash/List/Set/SortedSet (58 commands)
2. **Advanced string ops** — MGET, MSET, MSETNX, SETNX, APPEND, BITCOUNT, etc. (18 commands)
3. **Admin/monitoring** — CONFIG, INFO, SAVE, BGSAVE, PTTL, etc. (17 commands)
4. **Transactions** — MULTI, EXEC, DISCARD, WATCH, UNWATCH (5 commands)
5. **Pubsub** — SUBSCRIBE, etc. (documented as UNSUPPORTED by client)

## Test Strategy

Three-layer testing:
1. **Unit tests** — pure encoding/decoding, no runtime needed
2. **FakeConnection tests** — protocol layer, no network needed
3. **Integration tests** — live Redis, `may::run` + `may::go!`

See [[redis-command-e2e-test-coverage]] for full audit and implementation plan.
