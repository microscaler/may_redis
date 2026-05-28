# Story 7.2 — Hash Commands

**Objective:** Add hash manipulation commands beyond HSET/HGET. These cover field deletion, key enumeration, full hash retrieval, multi-field operations, and cursor-based iteration.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.1 (String Extension — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for hash commands), `docs/08-command-audit.md` (hash coverage)

## Functional Requirements

### FR-1: HDEL key field [field ...]
- Method: `hdel(&self, key: K, field: F) -> CommandBuilder` (single field)
- Method: `hdel_fields(&self, key: K, fields: &[impl ToRedisArgs]) -> CommandBuilder` (variadic)
- RESP: `HDEL key field1 [field2 ...]`
- Unit test: `test_command_hdel_encoding`

### FR-2: HKEYS key
- Method: `hkeys(&self, key: K) -> CommandBuilder`
- RESP: `HKEYS key`
- Unit test: `test_command_hkeys_encoding`

### FR-3: HGETALL key
- Method: `hgetall(&self, key: K) -> CommandBuilder`
- RESP: `HGETALL key`
- Unit test: `test_command_hgetall_encoding`

### FR-4: HMSET key field value [field value ...]
- Method: `hmset(&self, key: K, pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder`
- RESP: `HMSET key field1 value1 [field2 value2 ...]`
- Unit test: `test_command_hmset_encoding`

### FR-5: HINCRBY key field increment
- Method: `hincrby(&self, key: K, field: F, increment: i64) -> CommandBuilder`
- RESP: `HINCRBY key field increment`
- Unit test: `test_command_hincrby_encoding`

### FR-6: HLEN key
- Method: `hlen(&self, key: K) -> CommandBuilder`
- RESP: `HLEN key`
- Unit test: `test_command_hlen_encoding`

### FR-7: HEXISTS key field
- Method: `hexists(&self, key: K, field: F) -> CommandBuilder`
- RESP: `HEXISTS key field`
- Unit test: `test_command_hexists_encoding`

### FR-8: HSCAN key cursor [MATCH pattern] [COUNT count]
- Method: `hscan(&self, key: K, cursor: i64) -> CommandBuilder` (simple form)
- Method: `hscan_match(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder` (with match)
- RESP: `HSCAN key cursor [MATCH pattern] [COUNT count]`
- Unit tests: `test_command_hscan_encoding`, `test_command_hscan_match_encoding`

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing hash commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- HSCAN variadic args use `CommandBuilder::args()` for optional MATCH/COUNT parameters

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (after `append` method)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests`

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 8 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
