# Redis Command E2E Test Coverage — PRD

**Status:** Draft
**Author:** may-redis team
**Date:** 2025-06-01
**Scope:** Full end-to-end test coverage for all 122 Redis command methods across the `may-redis` crate.

---

## 1. Problem Statement

### Current State

- **122 command methods** defined across 8 traits (Strings, Hashes, Lists, Sets, SortedSets, Admin, Pubsub, Transactions)
- **100% protocol encoding coverage** — every command has RESP2 wire-format tests in `*_tests.rs` files
- **~12% real-data coverage** — integration tests cover only basic StringsCommands (SET, GET, DEL, EXISTS, INCR, TTL, EXPIRE) and a few AdminCommands (PING, DBSIZE, FLUSHDB)
- **5 out of 8 traits have ZERO integration tests**: Hashes (12 cmds), Lists (12 cmds), Sets (14 cmds), SortedSets (20 cmds), Transactions (5 cmds)
- **107 commands (88%) have no real-Redis tests** — only encoding tests, never executed against a live server

### Impact

- No confidence that complex commands work correctly over the may coroutine stack
- Hash/List/Set/SortedSet data structures are untested despite being critical for production workloads (perf tests use HSET, SADD, etc.)
- Transaction primitives (MULTI/EXEC/WATCH) have never been exercised
- No coverage of SCAN-family commands (critical for large datasets)
- No coverage of multi-key operations (MGET, MSET, MSETNX)
- In-memory test backend doesn't implement Hash/List/Set/SortedSet

---

## 2. Goals

1. **100% real-data coverage** — every command method exercised against a live Redis server via integration tests
2. **In-memory backend parity** — implement Hash/List/Set/SortedSet operations in `InMemoryClient` (feature `test`)
3. **E2E scenario coverage** — test realistic multi-command workflows, not just single-command roundtrips
4. **Concurrency coverage** — verify command ordering and response correlation under concurrent access for all command families
5. **Error-path coverage** — test type mismatches, server errors, protocol errors for each command family
6. **Maintain 350-line file limit** — all new test files follow the established splitting pattern
7. **0 regressions** — existing 319 tests must all pass

---

## 3. Architecture

### Test File Structure

Following the established pattern:

```
src/client/client_tests/
├── integration.rs          # Basic integration (existing, ~20 tests)
├── integration_strings.rs  # NEW: String command families
├── integration_hashes.rs   # NEW: Hash command families
├── integration_lists.rs    # NEW: List command families
├── integration_sets.rs     # NEW: Set command families
├── integration_sorted_sets.rs # NEW: SortedSet command families
├── integration_admin.rs    # NEW: Admin/monitoring commands
├── integration_transactions.rs # NEW: MULTI/EXEC/WATCH
├── integration_pubsub.rs   # NEW: Pubsub commands (encoding-only, client does NOT support subscribe)
├── integration_concurrency.rs # EXPAND: Multi-command-family concurrency
├── integration_scan.rs     # NEW: SCAN, HSCAN, SSCAN, ZSCAN
├── integration_pipeline.rs # EXPAND: Pipeline with all command families
└── integration_error_paths.rs # NEW: Type mismatches, server errors, protocol errors
```

### Test Patterns

Every test follows this structure:

```rust
#[test]
fn test_integration_<command_name>() {
    may::run(|| {
        may::go(|| {
            let client = RedisClient::connect("redis://127.0.0.1:6379").unwrap();
            client.execute(cmd("FLUSHDB")).unwrap();
            
            // Test logic: set up data, execute command, verify response
            
            client.execute(cmd("FLUSHDB")).unwrap();
        }).join();
    });
}
```

---

## 4. Test Scenarios by Command Family

### 4.1 StringsCommands (24 methods)

**Current coverage:** SET, GET, DEL, EXISTS, INCR, TTL, EXPIRE, KEYS, DBSIZE, PING, PUBLISH (~8 distinct commands)
**Missing:** 16 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| S1 | `test_integration_string_set_get_string` | SET, GET | Basic string store and retrieve |
| S2 | `test_integration_string_set_get_binary` | SET, GET | Binary data (non-UTF8) |
| S3 | `test_integration_string_set_overwrite` | SET, GET | Overwrite existing key |
| S4 | `test_integration_string_setnx` | SETNX | Set only if not exists |
| S5 | `test_integration_string_setex` | SETEX | Set with expiration |
| S6 | `test_integration_string_setex_expiration` | SETEX, TTL | Key expires at expected time |
| S7 | `test_integration_string_incr_by` | INCRBY | Atomic increment by value |
| S8 | `test_integration_string_decr` | DECR | Atomic decrement |
| S9 | `test_integration_string_decrby` | DECRBY | Atomic decrement by value |
| S10 | `test_integration_string_append` | APPEND | Append to existing value |
| S11 | `test_integration_string_strlen` | STRLEN | Get string length |
| S12 | `test_integration_string_getrange` | GETRANGE | Extract substring |
| S13 | `test_integration_string_setrange` | SETRANGE | Overwrite substring |
| S14 | `test_integration_string_mget` | MGET | Get multiple keys at once |
| S15 | `test_integration_string_mset` | MSET | Set multiple keys at once |
| S16 | `test_integration_string_msetnx` | MSETNX | Set multiple only if none exist |
| S17 | `test_integration_string_getbit_setbit` | GETBIT, SETBIT | Bit operations |
| S18 | `test_integration_string_bitcount` | BITCOUNT | Count set bits |
| S19 | `test_integration_string_bitcount_range` | BITCOUNT_RANGE | Count bits in range |
| S20 | `test_integration_string_exists_type` | EXISTS, TYPE | Check key existence and type |

---

### 4.2 HashesCommands (12 methods)

**Current coverage:** 0 commands (zero integration tests)
**Missing:** All 12 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| H1 | `test_integration_hash_hset_hget` | HSET, HGET | Store and retrieve single field |
| H2 | `test_integration_hash_hmset_hget` | HMSET, HGET | Set multiple fields, get single |
| H3 | `test_integration_hash_hgetall` | HGETALL | Retrieve all fields and values |
| H4 | `test_integration_hash_hdel` | HDEL | Delete single field |
| H5 | `test_integration_hash_hdel_multi` | HDEL | Delete multiple fields |
| H6 | `test_integration_hash_hkeys` | HKEYS | Get all field names |
| H7 | `test_integration_hash_hvals` | HVALS | Get all values |
| H8 | `test_integration_hash_hlen` | HLEN | Get field count |
| H9 | `test_integration_hash_hexists` | HEXISTS | Check field existence |
| H10 | `test_integration_hash_hincrby` | HINCRBY | Increment integer field |
| H11 | `test_integration_hash_hscan` | HSCAN | Incremental iteration (no pattern) |
| H12 | `test_integration_hash_hscan_match` | HSCAN_MATCH | Incremental iteration with pattern |
| H13 | `test_integration_hash_hash_empty` | HGETALL, HKEYS, HVALS, HLEN | Behavior on non-existent key |
| H14 | `test_integration_hash_hash_large` | HSET, HGETALL | Large hash (1000+ fields) |

---

### 4.3 ListsCommands (12 methods)

**Current coverage:** 0 commands (zero integration tests)
**Missing:** All 12 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| L1 | `test_integration_list_lpush_rpush` | LPUSH, RPUSH | Push to front and back |
| L2 | `test_integration_list_lpop_rpop` | LPOP, RPOP | Pop from front and back |
| L3 | `test_integration_list_lrange` | LRANGE | Get range of elements |
| L4 | `test_integration_list_lindex` | LINDEX | Get element by index |
| L5 | `test_integration_list_llen` | LLEN | Get list length |
| L6 | `test_integration_list_lset` | LSET | Set element by index |
| L7 | `test_integration_list_lrem` | LREM | Remove by value and count |
| L8 | `test_integration_list_ltrim` | LTRIM | Trim list to range |
| L9 | `test_integration_list_list_empty` | LPOP, LLEN | Behavior on empty list |
| L10 | `test_integration_list_list_push_pop_cycle` | LPUSH, LPOP, RPUSH, RPOP | Push-pop cycle preserves order |
| L11 | `test_integration_list_list_large` | LPUSH, LRANGE | Large list (10000 elements) |

---

### 4.4 SetsCommands (14 methods)

**Current coverage:** SISMEMBER only (via perf tests, but no integration test)
**Missing:** 13 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| ST1 | `test_integration_set_sadd_smembers` | SADD, SMEMBERS | Add members, retrieve all |
| ST2 | `test_integration_set_srem` | SREM | Remove member |
| ST3 | `test_integration_set_sismember` | SISMEMBER | Check membership |
| ST4 | `test_integration_set_scard` | SCARD | Get set cardinality |
| ST5 | `test_integration_set_spop` | SPOP | Pop single random member |
| ST6 | `test_integration_set_spop_count` | SPOP | Pop N random members |
| ST7 | `test_integration_set_srandmember` | SRANDMEMBER | Get random member without removing |
| ST8 | `test_integration_set_srandmember_count` | SRANDMEMBER | Get N random members without removing |
| ST9 | `test_integration_set_sinter` | SINTER | Intersection of two sets |
| ST10 | `test_integration_set_sunion` | SUNION | Union of two sets |
| ST11 | `test_integration_set_smove` | SMOVE | Move member between sets |
| ST12 | `test_integration_set_sscan` | SSCAN | Incremental iteration |
| ST13 | `test_integration_set_sscan_match` | SSCAN_MATCH | Incremental iteration with pattern |
| ST14 | `test_integration_set_set_empty` | SMEMBERS, SCARD | Behavior on non-existent key |
| ST15 | `test_integration_set_set_duplicates` | SADD | Adding duplicates has no effect |

---

### 4.5 SortedSetsCommands (20 methods)

**Current coverage:** 0 commands (zero integration tests)
**Missing:** All 20 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| SZ1 | `test_integration_sortedset_zadd_zscore` | ZADD, ZSCORE | Add member with score, retrieve score |
| ST2 | `test_integration_sortedset_zadd_multi` | ZADD | Add multiple members with scores |
| ST3 | `test_integration_sortedset_zrange` | ZRANGE | Get range by index (no scores) |
| ST4 | `test_integration_sortedset_zrange_withscores` | ZRANGE_WITHSCORES | Get range by index (with scores) |
| ST5 | `test_integration_sortedset_zrangebyscore` | ZRANGEBYSCORE | Get range by score |
| ST6 | `test_integration_sortedset_zrangebyscore_withscores` | ZRANGEBYSCORE_WITHSCORES | Get range by score (with scores) |
| ST7 | `test_integration_sortedset_zrangebyscore_limit` | ZRANGEBYSCORE | Get range with LIMIT |
| ST8 | `test_integration_sortedset_zrank_zrevrank` | ZRANK, ZREVRANK | Get rank of member |
| ST9 | `test_integration_sortedset_zrem` | ZREM | Remove single member |
| ST10 | `test_integration_sortedset_zrem_multi` | ZREM_MEMBERS | Remove multiple members |
| ST11 | `test_integration_sortedset_zcard` | ZCARD | Get sorted set cardinality |
| ST12 | `test_integration_sortedset_zcount` | ZCOUNT | Count members in score range |
| ST13 | `test_integration_sortedset_zincrby` | ZINCRBY | Increment member's score |
| ST14 | `test_integration_sortedset_zpopmax` | ZPOPMAX | Pop highest score |
| ST15 | `test_integration_sortedset_zpopmax_count` | ZPOPMAX | Pop N highest scores |
| ST16 | `test_integration_sortedset_zpopmin` | ZPOPMIN | Pop lowest score |
| ST17 | `test_integration_sortedset_zpopmin_count` | ZPOPMIN | Pop N lowest scores |
| ST18 | `test_integration_sortedset_zscan` | ZSCAN | Incremental iteration |
| ST19 | `test_integration_sortedset_zscan_match` | ZSCAN_MATCH | Incremental iteration with pattern |
| ST20 | `test_integration_sortedset_sortedset_empty` | ZRANGE, ZCARD | Behavior on non-existent key |
| ST21 | `test_integration_sortedset_duplicate_scores` | ZADD | Same score for multiple members |
| ST22 | `test_integration_sortedset_large` | ZADD, ZRANGE | Large sorted set (10000 members) |

---

### 4.6 AdminCommands (28 methods)

**Current coverage:** PING, KEYS, DBSIZE, FLUSHDB (~4 commands)
**Missing:** 24 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| A1 | `test_integration_admin_type` | TYPE | Check key type |
| A2 | `test_integration_admin_move` | MOVE | Move key to another DB |
| A3 | `test_integration_admin_rename` | RENAME | Rename key |
| A4 | `test_integration_admin_renamenx` | RENAMENX | Rename only if target doesn't exist |
| A5 | `test_integration_admin_touch` | TOUCH | Touch keys (update access time) |
| A6 | `test_integration_admin_pttl` | PTTL | Get remaining TTL in milliseconds |
| A7 | `test_integration_admin_pexpire` | PEXPIRE | Set expiration in milliseconds |
| A8 | `test_integration_admin_pexpireat` | PEXPIREAT | Set expiration timestamp in ms |
| A9 | `test_integration_admin_persist` | PERSIST | Remove expiration |
| A10 | `test_integration_admin_save` | SAVE | Synchronous save |
| A11 | `test_integration_admin_bgsave` | BGSAVE | Asynchronous save |
| A12 | `test_integration_admin_info` | INFO | Server info (basic check) |
| A13 | `test_integration_admin_info_section` | INFO_SECTION | Server info for specific section |
| A14 | `test_integration_admin_config_get` | CONFIG_GET | Get config value |
| A15 | `test_integration_admin_flushall` | FLUSHALL | Flush all databases |
| A16 | `test_integration_admin_auth` | AUTH | Auth with password |
| A17 | `test_integration_admin_select` | SELECT | Select database |
| A18 | `test_integration_admin_scan` | SCAN | Incremental key iteration |
| A19 | `test_integration_admin_scan_match` | SCAN_MATCH | Incremental key iteration with pattern |

---

### 4.7 TransactionsCommands (5 methods)

**Current coverage:** 0 commands (zero integration tests)
**Missing:** All 5 commands

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| T1 | `test_integration_transaction_multi_exec` | MULTI, EXEC | Basic transaction |
| T2 | `test_integration_transaction_multi_exec_error` | MULTI, EXEC | Transaction where one command fails |
| T3 | `test_integration_transaction_discard` | MULTI, DISCARD | Abort transaction |
| T4 | `test_integration_transaction_watch` | WATCH, MULTI, EXEC | Optimistic locking |
| T5 | `test_integration_transaction_unwatch` | UNWATCH, MULTI, EXEC | Cancel watch |
| T6 | `test_integration_transaction_watch_conflict` | WATCH, MULTI, EXEC | Conflict between watch and execute |

---

### 4.8 PubsubCommands (7 methods)

**Current coverage:** PUBLISH only (~1 command)
**Missing:** 6 methods (SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE are documented as UNSUPPORTED)

#### Test Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| P1 | `test_integration_pubsub_publish` | PUBLISH | Publish message |
| P2 | `test_integration_pubsub_pubsub_subscriber` | PUBLISH | Publish to non-existent subscriber |
| P3 | `test_integration_pubsub_pubsub_nopub` | PUBSUB_NUMSUB | No subscribers |

**Note:** SUBSCRIBE/PSUBSCRIBE are intentionally unsupported. The client uses request-response correlation (monotonic tags) which is incompatible with pub/sub's push model. Tests verify PUBLISH works correctly and document the limitation.

---

### 4.9 Concurrency & Pipeline Scenarios (Expansion)

Current concurrency tests only cover basic SET/GET. Expand to cover all command families.

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| C1 | `test_integration_concurrent_hashes` | HSET, HGET | Concurrent hash operations |
| C2 | `test_integration_concurrent_lists` | LPUSH, LPOP | Concurrent list operations |
| C3 | `test_integration_concurrent_sets` | SADD, SMEMBERS | Concurrent set operations |
| C4 | `test_integration_concurrent_sortedsets` | ZADD, ZRANGE | Concurrent sorted set operations |
| C5 | `test_integration_pipeline_all_command_families` | All | Pipeline with mixed command families |
| C6 | `test_integration_pipeline_hashes` | HSET, HGETALL | Pipeline hash commands |
| C7 | `test_integration_pipeline_lists` | LPUSH, LRANGE | Pipeline list commands |
| C8 | `test_integration_pipeline_sets` | SADD, SMEMBERS | Pipeline set commands |
| C9 | `test_integration_pipeline_sortedsets` | ZADD, ZRANGE | Pipeline sorted set commands |

---

### 4.10 Error-Path Scenarios

| # | Test Name | Commands | Scenario |
|---|-----------|----------|----------|
| E1 | `test_integration_error_wrong_type_hash` | HSET on string key | Type mismatch error |
| E2 | `test_integration_error_wrong_type_list` | LPUSH on set key | Type mismatch error |
| E3 | `test_integration_error_wrong_type_set` | SADD on hash key | Type mismatch error |
| E4 | `test_integration_error_wrong_type_sortedset` | ZADD on list key | Type mismatch error |
| E5 | `test_integration_error_invalid_encoding` | Invalid RESP response | Protocol error handling |
| E6 | `test_integration_error_server_oom` | OOM error simulation | Server error propagation |
| E7 | `test_integration_error_connection_reset_during_hash` | HSET during disconnect | Connection reset handling |
| E8 | `test_integration_error_timeout_during_scan` | SCAN with timeout | Timeout during scan |

---

## 5. In-Memory Backend Extension

The `InMemoryClient` (feature `test`) currently only implements: SET, GET, DEL, EXISTS, INCR, TTL, EXPIRE, KEYS, DBSIZE, FLUSHDB.

### Required Additions

| Module | Commands to Implement |
|--------|----------------------|
| `src/client/in_memory.rs` | HGET, HGETALL, HKEYS, HDEL, HSET, HMSET, HLEN, HEXISTS, HINCRBY, HVALS |
| `src/client/in_memory.rs` | LPUSH, RPUSH, LPOP, RPOP, LRANGE, LINDEX, LLEN, LSET, LREM, LTRIM |
| `src/client/in_memory.rs` | SADD, SREM, SMEMBERS, SISMEMBER, SCARD, SPOP, SRANDMEMBER, SINTER, SUNION, SMOVE |
| `src/client/in_memory.rs` | ZADD, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN |
| `src/client/in_memory.rs` | MGET, MSET, MSETNX, SETNX, SETEX, APPEND, GETRANGE, SETRANGE, STRLEN |
| `src/client/in_memory.rs` | HSCAN, SSCAN, ZSCAN (stub returns empty, for encoding test coverage) |

Each new data structure needs its own storage type:

```rust
enum InMemoryValue {
    String(String),
    Hash(BTreeMap<String, String>),  // New
    List(Vec<String>),               // New
    HashSet(HashSet<String>),        // New
    SortedSet(Vec<(f64, String)>),   // New (sorted by score)
    Expired,
}
```

### In-Memory Test Coverage Goals

| # | Test Name | Coverage |
|---|-----------|----------|
| IM1 | `test_inmem_hash_set_get` | HSET/HGET on in-memory backend |
| IM2 | `test_inmem_hash_getall` | HGETALL on in-memory backend |
| IM3 | `test_inmem_hash_del` | HDEL on in-memory backend |
| IM4 | `test_inmem_list_push_pop` | LPUSH/LPOP/RPUSH/RPOP on in-memory backend |
| IM5 | `test_inmem_list_range` | LRANGE on in-memory backend |
| IM6 | `test_inmem_set_add_remove` | SADD/SREM on in-memory backend |
| IM7 | `test_inmem_set_members` | SMEMBERS/SCARD on in-memory backend |
| IM8 | `test_inmem_sortedset_add_score` | ZADD/ZSCORE on in-memory backend |
| IM9 | `test_inmem_sortedset_range` | ZRANGE/ZCARD on in-memory backend |
| IM10 | `test_inmem_multi_key_ops` | MGET/MSET/MSETNX on in-memory backend |

---

## 6. Implementation Plan

### Phase 1: Core Data Structures (Priority P0)

**Goal:** Hash, List, Set, SortedSet integration tests

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 1 | `integration_hashes.rs` — 14 tests for all HashCommands | `src/client/client_tests/integration_hashes.rs` | ~350 |
| 2 | `integration_lists.rs` — 11 tests for all ListsCommands | `src/client/client_tests/integration_lists.rs` | ~280 |
| 3 | `integration_sets.rs` — 15 tests for all SetsCommands | `src/client/client_tests/integration_sets.rs` | ~380 |
| 4 | `integration_sorted_sets.rs` — 22 tests for all SortedSetsCommands | `src/client/client_tests/integration_sorted_sets.rs` | ~450 |

### Phase 2: Admin & Transactions (Priority P1)

**Goal:** Admin and transaction integration tests

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 5 | `integration_admin.rs` — 19 tests for AdminCommands | `src/client/client_tests/integration_admin.rs` | ~400 |
| 6 | `integration_transactions.rs` — 6 tests for TransactionsCommands | `src/client/client_tests/integration_transactions.rs` | ~200 |

### Phase 3: Strings Extension (Priority P2)

**Goal:** Complete String command coverage

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 7 | `integration_strings.rs` — 20 tests for remaining StringCommands | `src/client/client_tests/integration_strings.rs` | ~500 |

### Phase 4: Concurrency & Pipeline (Priority P2)

**Goal:** Multi-command-family concurrency tests

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 8 | Expand `integration_concurrency.rs` with hash/list/set/sortedset scenarios | `src/client/client_tests/integration_concurrency.rs` | ~300 |
| 9 | Expand `integration_pipeline.rs` with mixed-family pipeline tests | `src/client/client_tests/integration_pipeline.rs` | ~250 |

### Phase 5: Error Paths & Scanning (Priority P2)

**Goal:** Error handling and scan coverage

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 10 | `integration_scan.rs` — SCAN, HSCAN, SSCAN, ZSCAN | `src/client/client_tests/integration_scan.rs` | ~200 |
| 11 | `integration_error_paths.rs` — 8 type-mismatch and server error tests | `src/client/client_tests/integration_error_paths.rs` | ~200 |

### Phase 6: In-Memory Backend (Priority P3)

**Goal:** In-memory backend parity for all data structures

| Step | Task | File | Est. Lines |
|------|------|------|-----------|
| 12 | Extend `in_memory.rs` with Hash/List/Set/SortedSet storage types | `src/client/in_memory.rs` | +200 |
| 13 | `in_memory_tests.rs` — 10 in-memory backend tests | `src/client/in_memory_tests.rs` | ~250 |

---

## 7. Acceptance Criteria

1. **All 122 command methods** have at least one integration test exercising them against a live Redis server
2. **319 existing tests** all pass (no regressions)
3. **All new test files** are under 350 lines (split as needed)
4. **`cargo clippy --lib --tests --all-features -- -D warnings`** passes cleanly
5. **`cargo fmt --all --check`** passes cleanly
6. **In-memory backend** supports at minimum: HGET/HGETALL/HDEL, LPUSH/LPOP, SADD/SMEMBERS, ZADD/ZRANGE
7. **`scripts/check-file-lengths.sh`** passes (all production files under 350 lines)

---

## 8. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Redis server availability in CI | Medium | High | Use GitHub Actions Redis service container (already configured) |
| Test flakes from timing issues (TTL, scan) | Medium | Medium | Use `may::coroutine::yield_now()` delays; retry loops for scan |
| In-memory backend complexity | High | Medium | Implement incrementally; stub complex ops first |
| Sorted set ordering edge cases | Medium | Low | Test duplicate scores, equal scores, empty sets explicitly |
| Transaction isolation semantics | Low | High | Focus on basic MULTI/EXEC; WATCH/UNWATCH secondary |
| Pubsub SUBSCRIBE unsupported | N/A | Low | Document in test comments; only test PUBLISH |

---

## 9. Out of Scope

- **Pubsub SUBSCRIBE/UNSUBSCRIBE/PSUBSCRIBE/PUNSUBSCRIBE** — explicitly unsupported by may-redis (request-response correlation incompatible with push model)
- **SHUTDOWN, SAVE, BGSAVE** — server management commands (not typically used programmatically)
- **CONFIG SET** — dangerous; read-only CONFIG GET only
- **CLUSTER commands** — may-redis is a standalone Redis client, not cluster-aware
- **RESP3 types** — may-redis v1 is RESP2 only

---

## 10. Metrics Dashboard

### Before Implementation

| Metric | Value |
|--------|-------|
| Total command methods | 122 |
| Commands with encoding tests | 122 (100%) |
| Commands with integration tests | ~15 (12%) |
| Traits with zero integration tests | 5/8 (63%) |
| Total integration tests | ~25 |

### Target After Implementation

| Metric | Value |
|--------|-------|
| Total command methods | 122 |
| Commands with encoding tests | 122 (100%) |
| Commands with integration tests | 115 (~94%)* |
| Traits with zero integration tests | 1/8 (Pubsub) |
| Total integration tests | ~130+ |

*Pubsub commands (SUBSCRIBE, etc.) intentionally excluded — PUBLISH (1/7 pubsub methods) is tested.

---

## 11. Review Checklist

When implementing this PRD, reviewers should verify:

- [ ] Each test exercises the command against a live Redis server (not just encoding)
- [ ] FLUSHDB is called before and after each test for isolation
- [ ] Tests use `may::run`/`may::go!` pattern (no tokio)
- [ ] No `unwrap()`/`expect()` in test logic (use `assert_eq!` with expected values)
- [ ] Concurrency tests use `--test-threads=1` flag
- [ ] File sizes stay under 350 lines
- [ ] In-memory backend operations match Redis semantics exactly
- [ ] Error tests verify the correct `RedisError` variant is returned
- [ ] No integration tests depend on data from other tests
