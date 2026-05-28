# Story 7.5 — Sorted Set Commands

**Objective:** Add sorted set operations. Sorted sets are used for leaderboards, ranking, time-ordered data, and priority queues. This is the most complex command group due to multiple numeric arguments and score-based range queries.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.4 (List — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for sorted set commands), `docs/08-command-audit.md` (sorted set coverage)

## Functional Requirements

### FR-1: ZADD key score member [score member ...]
- Method: `zadd(&self, key: K, score: f64, member: M) -> CommandBuilder` (single)
- Method: `zadd_multi(&self, key: K, scores: &[(f64, impl ToRedisArgs)]) -> CommandBuilder` (batch)
- RESP: `ZADD key score1 member1 [score2 member2 ...]`
- Unit tests: `test_command_zadd_encoding`, `test_command_zadd_multi_encoding`

### FR-2: ZREM key member [member ...]
- Method: `zrem(&self, key: K, member: M) -> CommandBuilder` (single)
- Method: `zrem_members(&self, key: K, members: &[impl ToRedisArgs]) -> CommandBuilder` (variadic)
- RESP: `ZREM key member [member ...]`
- Unit tests: `test_command_zrem_encoding`, `test_command_zrem_members_encoding`

### FR-3: ZRANGE key start stop [WITHSCORES]
- Method: `zrange(&self, key: K, start: i64, stop: i64) -> CommandBuilder`
- Method: `zrange_withscores(&self, key: K, start: i64, stop: i64) -> CommandBuilder`
- RESP: `ZRANGE key start stop` / `ZRANGE key start stop WITHSCORES`
- Unit tests: `test_command_zrange_encoding`, `test_command_zrange_withscores_encoding`

### FR-4: ZRANK key member
- Method: `zrank(&self, key: K, member: M) -> CommandBuilder`
- RESP: `ZRANK key member`
- Unit test: `test_command_zrank_encoding`

### FR-5: ZSCORE key member
- Method: `zscore(&self, key: K, member: M) -> CommandBuilder`
- RESP: `ZSCORE key member`
- Unit test: `test_command_zscore_encoding`

### FR-6: ZCARD key
- Method: `zcard(&self, key: K) -> CommandBuilder`
- RESP: `ZCARD key`
- Unit test: `test_command_zcard_encoding`

### FR-7: ZCOUNT key min max
- Method: `zcount(&self, key: K, min: f64, max: f64) -> CommandBuilder`
- RESP: `ZCOUNT key min max`
- Unit test: `test_command_zcount_encoding`

### FR-8: ZINCRBY key increment member
- Method: `zincrby(&self, key: K, increment: f64, member: M) -> CommandBuilder`
- RESP: `ZINCRBY key increment member`
- Unit test: `test_command_zincrby_encoding`

### FR-9: ZPOPMAX key [count]
- Method: `zpopmax(&self, key: K) -> CommandBuilder` (single)
- Method: `zpopmax_count(&self, key: K, count: i64) -> CommandBuilder` (batch)
- RESP: `ZPOPMAX key` / `ZPOPMAX key count`
- Unit tests: `test_command_zpopmax_encoding`, `test_command_zpopmax_count_encoding`

### FR-10: ZPOPMIN key [count]
- Method: `zpopmin(&self, key: K) -> CommandBuilder` (single)
- Method: `zpopmin_count(&self, key: K, count: i64) -> CommandBuilder` (batch)
- RESP: `ZPOPMIN key` / `ZPOPMIN key count`
- Unit tests: `test_command_zpopmin_encoding`, `test_command_zpopmin_count_encoding`

### FR-11: ZSCAN key cursor [MATCH pattern] [COUNT count]
- Method: `zscan(&self, key: K, cursor: i64) -> CommandBuilder` (simple form)
- Method: `zscan_match(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder` (with match)
- RESP: `ZSCAN key cursor [MATCH pattern] [COUNT count]`
- Unit tests: `test_command_zscan_encoding`, `test_command_zscan_match_encoding`

### FR-12: ZRANGEBYSCORE key min max [WITHSCORES] [LIMIT offset count]
- Method: `zrangebyscore(&self, key: K, min: f64, max: f64) -> CommandBuilder` (simple form)
- Method: `zrangebyscore_withscores(&self, key: K, min: f64, max: f64) -> CommandBuilder`
- Method: `zrangebyscore_limit(&self, key: K, min: f64, max: f64, offset: i64, count: i64) -> CommandBuilder`
- RESP: `ZRANGEBYSCORE key min max` / `ZRANGEBYSCORE key min max WITHSCORES` / `ZRANGEBYSCORE key min max LIMIT offset count`
- Unit tests: `test_command_zrangebyscore_encoding`, `test_command_zrangebyscore_withscores_encoding`, `test_command_zrangebyscore_limit_encoding`

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- Commands with variadic args (ZRANGE WITHSCORES, ZRANGEBYSCORE LIMIT) append optional flags as additional `.arg()` calls
- f64 scores are converted to string via `.to_string()` — no new float-specific ToRedisArgs impl needed

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 12 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
- [ ] Score formatting uses space-separated decimal (e.g., "1.5" not "1,5")
