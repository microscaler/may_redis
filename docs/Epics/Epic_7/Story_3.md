# Story 7.3 — Set Commands

**Objective:** Add set operations beyond SADD/SREM/SISMEMBER. These include enumeration, set algebra (intersection, union, difference), element removal, and cursor-based iteration.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.2 (Hash — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for set commands), `docs/08-command-audit.md` (set coverage)

## Functional Requirements

### FR-1: SMEMBERS key
- Method: `smembers(&self, key: K) -> CommandBuilder`
- RESP: `SMEMBERS key`
- Unit test: `test_command_smembers_encoding`

### FR-2: SPOP key [count]
- Method: `spop(&self, key: K) -> CommandBuilder` (pop single)
- Method: `spop_count(&self, key: K, count: i64) -> CommandBuilder` (pop n)
- RESP: `SPOP key` / `SPOP key count`
- Unit tests: `test_command_spop_encoding`, `test_command_spop_count_encoding`

### FR-3: SRANDMEMBER key [count]
- Method: `srandmember(&self, key: K) -> CommandBuilder` (single)
- Method: `srandmember_count(&self, key: K, count: i64) -> CommandBuilder` (n)
- RESP: `SRANDMEMBER key` / `SRANDMEMBER key count`
- Unit tests: `test_command_srandmember_encoding`, `test_command_srandmember_count_encoding`

### FR-4: SCARD key
- Method: `scard(&self, key: K) -> CommandBuilder`
- RESP: `SCARD key`
- Unit test: `test_command_scard_encoding`

### FR-5: SINTER key [key ...]
- Method: `sinter(&self, keys: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `SINTER key1 key2 ...`
- Unit test: `test_command_sinter_encoding`

### FR-6: SUNION key [key ...]
- Method: `sunion(&self, keys: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `SUNION key1 key2 ...`
- Unit test: `test_command_sunion_encoding`

### FR-7: SMOVE source destination member
- Method: `smove(&self, source: K, destination: K, member: M) -> CommandBuilder`
- RESP: `SMOVE source destination member`
- Unit test: `test_command_smove_encoding`

### FR-8: SSCAN key cursor [MATCH pattern] [COUNT count]
- Method: `sscan(&self, key: K, cursor: i64) -> CommandBuilder` (simple form)
- Method: `sscan_match(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder` (with match)
- RESP: `SSCAN key cursor [MATCH pattern] [COUNT count]`
- Unit tests: `test_command_sscan_encoding`, `test_command_sscan_match_encoding`

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing set commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- Commands with variadic keys (SINTER, SUNION) use `CommandBuilder::args()`

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (after `srem` method)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests`

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 8 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
