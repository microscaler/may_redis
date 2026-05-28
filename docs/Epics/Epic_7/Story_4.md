# Story 7.4 — List Commands

**Objective:** Add list manipulation commands. Lists are used for queues, pipelines, and ordered data structures. Commands cover push/pop operations, range queries, element access, and blocking operations.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.3 (Set — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for list commands), `docs/08-command-audit.md` (list coverage)

## Functional Requirements

### FR-1: LPUSH key value [value ...]
- Method: `lpush(&self, key: K, values: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `LPUSH key value1 [value2 ...]`
- Unit test: `test_command_lpush_encoding`

### FR-2: RPUSH key value [value ...]
- Method: `rpush(&self, key: K, values: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `RPUSH key value1 [value2 ...]`
- Unit test: `test_command_rpush_encoding`

### FR-3: LPOP key
- Method: `lpop(&self, key: K) -> CommandBuilder`
- RESP: `LPOP key`
- Unit test: `test_command_lpop_encoding`

### FR-4: RPOP key
- Method: `rpop(&self, key: K) -> CommandBuilder`
- RESP: `RPOP key`
- Unit test: `test_command_rpop_encoding`

### FR-5: LLEN key
- Method: `llen(&self, key: K) -> CommandBuilder`
- RESP: `LLEN key`
- Unit test: `test_command_llen_encoding`

### FR-6: LRANGE key start stop
- Method: `lrange(&self, key: K, start: i64, stop: i64) -> CommandBuilder`
- RESP: `LRANGE key start stop`
- Unit test: `test_command_lrange_encoding`

### FR-7: LINDEX key index
- Method: `lindex(&self, key: K, index: i64) -> CommandBuilder`
- RESP: `LINDEX key index`
- Unit test: `test_command_lindex_encoding`

### FR-8: LSET key index value
- Method: `lset(&self, key: K, index: i64, value: V) -> CommandBuilder`
- RESP: `LSET key index value`
- Unit test: `test_command_lset_encoding`

### FR-9: LREM key count value
- Method: `lrem(&self, key: K, count: i64, value: V) -> CommandBuilder`
- RESP: `LREM key count value`
- Unit test: `test_command_lrem_encoding`

### FR-10: LTRIM key start stop
- Method: `ltrim(&self, key: K, start: i64, stop: i64) -> CommandBuilder`
- RESP: `LTRIM key start stop`
- Unit test: `test_command_ltrim_encoding`

### FR-11: BLPOP key [key ...] timeout
- Method: `blpop(&self, keys: &[impl ToRedisArgs], timeout: i64) -> CommandBuilder`
- RESP: `BLPOP key1 [key2 ...] timeout`
- Unit test: `test_command_blpop_encoding`

### FR-12: BRPOP key [key ...] timeout
- Method: `brpop(&self, keys: &[impl ToRedisArgs], timeout: i64) -> CommandBuilder`
- RESP: `BRPOP key1 [key2 ...] timeout`
- Unit test: `test_command_brpop_encoding`

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- Commands with variadic keys (LPUSH, RPUSH, BLPOP, BRPOP) use `CommandBuilder::args()`

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (after `append` method)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests`

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 12 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
