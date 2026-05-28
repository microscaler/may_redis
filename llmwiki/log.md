# Wiki Log

> Chronological record of all wiki actions. Append-only.
> Format: `## [YYYY-MM-DD] action | subject`

## [2026-06-01] create | Wiki initialized
- Domain: Project infrastructure, architecture, and engineering decisions
- Structure created with SCHEMA.md, index.md, log.md

## [2026-06-01] update | Redis command coverage audit
- Created comparison page: redis-command-coverage.md
- Documents 20/82+ commands implemented (~24% coverage)
- Confirms 100% sesame-idam command coverage (all 11 canonical commands implemented)
- Lists 60+ missing commands by category (HASH, SET, LIST, SORTED SET, STRING EXTENSION, PUB/SUB, TRANSACTIONS, SERVER/ADMIN)

## [2026-06-01] create | Redis command expansion epic
- Created Epic 7 stories for systematic Redis command expansion
- Story 1: String Extension (DECR, SETNX, MGET, MSET, STRLEN, GETRANGE, SETBIT, GETBIT, BITCOUNT)
- Story 2: Hash (HDEL, HKEYS, HGETALL, HMSET, HINCRBY, HLEN, HEXISTS, HSCAN)
- Story 3: Set (SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION, SMOVE, SSCAN)
- Story 4: List (LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP)
- Story 5: Sorted Set (ZADD, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZSCAN, ZRANGEBYSCORE)
- Story 6: Pub/Sub + Transactions (SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE, MULTI, EXEC, DISCARD, WATCH, UNWATCH)
- Story 7: Server/Admin (SELECT, TYPE, MOVE, RENAME, RENAMENX, SORT, SCAN, TOUCH, SAVE, BGSAVE, FLUSHALL, PTTL, PEXPIRE, PERSIST, SHUTDOWN, INFO, CONFIG)

## [2026-06-01] update | Epic 7 Story 1 implemented
- Added 13 String Extension commands to `src/protocol/commands.rs`: DECR, DECRBY, SETNX, MGET, MSET, MSETNX, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT (basic), BITCOUNT_RANGE
- Added 13 corresponding unit tests verifying RESP2 wire encoding
- Total Commands trait methods: 22 → 35
- Total command tests: 21 → 34
- All 35 tests pass (lib only, no runtime needed)
- Clippy --lib: zero warnings

## [2026-06-01] update | Epic 7 Story 1 verified complete
- All 13 checklist items verified against Story_1.md requirements
- `cargo check --lib`: PASS
- All 13 specific command tests pass: DECR, DECRBY, SETNX, MGET, MSET, MSETNX, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, BITCOUNT_RANGE
- `cargo clippy --lib -- -D warnings`: zero warnings
- Coverage: 35/82+ commands (~43%), sesame-idam still 100% covered
- Story 7.2 (Hash): 0 commands implemented, still PENDING — 10 hash extension commands remain

## [2026-06-01] fix | Zero clippy warnings across entire codebase
- Fixed 63 clippy errors in `tests/perf/main.rs`:
  - `unreadable_literal`: 4 occurrences of `2592000` → `2_592_000`
  - `unused-variables`: `jti_value` → `_jti_value`, `elapsed_ms` → `_elapsed_ms`
  - `format_collect`: replaced `.map().collect()` fold pattern with `fold(String::new(), |mut acc, _| write!(...) ... acc)`
  - `uninlined-format-args`: ~35 `format!("...", var)` → `format!("{var}")` inline patterns
  - `manual-div-ceil`: `(count + workers - 1) / workers` → `count.div_ceil(workers)`
  - `cast_lossless`: `i32 as f64` → `f64::from(i32)` for all ops_per_sec calculations
  - `unused-must-use`: added `let _ =` to 12 unchecked `client.execute()` calls
  - `needless-borrows-for-generic-args`: removed `&` before `format!(...)` in `client.get()` calls
  - `no-effect-underscore-binding`: `_ops_per_sec` already ignored, fixed cast_lossless
  - `manual-range-patterns`: `10 | 11 | 12 | 13` → `10..=13`, `14 | 15 | 16` → `14..=16`
  - Added `use std::fmt::Write` for `write!` macro in `random_hex()`
- All 35 command tests pass, clippy --lib --tests --all-features: ZERO warnings
