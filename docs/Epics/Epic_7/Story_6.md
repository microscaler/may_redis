# Story 7.6 — Pub/Sub and Transaction Commands

**Objective:** Add pub/sub subscription commands and transactional commands. These require understanding of different RESP response types (pub/sub returns arrays of status messages, transactions use multi-bulk response arrays).

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.5 (Sorted Set — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for pub/sub and transaction commands), `docs/08-command-audit.md` (pub/sub and transaction coverage)

## Functional Requirements — Pub/Sub

### FR-1: SUBSCRIBE channel [channel ...]
- Method: `subscribe(&self, channels: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `SUBSCRIBE channel1 [channel2 ...]`
- Unit test: `test_command_subscribe_encoding`
- Note: This is a special command — after sending, the connection enters pub/sub mode. The `may-redis` connection layer needs to handle the different response format.

### FR-2: UNSUBSCRIBE [channel [channel ...]]
- Method: `unsubscribe(&self) -> CommandBuilder` (unsubscribe all)
- Method: `unsubscribe_channels(&self, channels: &[impl ToRedisArgs]) -> CommandBuilder` (specific)
- RESP: `UNSUBSCRIBE` / `UNSUBSCRIBE channel1 [channel2 ...]`
- Unit tests: `test_command_unsubscribe_encoding`, `test_command_unsubscribe_channels_encoding`

### FR-3: PSUBSCRIBE pattern [pattern ...]
- Method: `psubscribe(&self, patterns: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `PSUBSCRIBE pattern1 [pattern2 ...]`
- Unit test: `test_command_psubscribe_encoding`

### FR-4: PUNSUBSCRIBE [pattern [pattern ...]]
- Method: `punsubscribe(&self) -> CommandBuilder` (unsubscribe all)
- Method: `punsubscribe_patterns(&self, patterns: &[impl ToRedisArgs]) -> CommandBuilder` (specific)
- RESP: `PUNSUBSCRIBE` / `PUNSUBSCRIBE pattern1 [pattern2 ...]`
- Unit tests: `test_command_punsubscribe_encoding`, `test_command_punsubscribe_patterns_encoding`

## Functional Requirements — Transactions

### FR-5: MULTI
- Method: `multi(&self) -> CommandBuilder`
- RESP: `MULTI`
- Unit test: `test_command_multi_encoding`
- Note: Enters transaction mode. All subsequent commands are queued.

### FR-6: EXEC
- Method: `exec(&self) -> CommandBuilder`
- RESP: `EXEC`
- Unit test: `test_command_exec_encoding`
- Note: Executes the queued transaction. Returns an array of all results.

### FR-7: DISCARD
- Method: `discard(&self) -> CommandBuilder`
- RESP: `DISCARD`
- Unit test: `test_command_discard_encoding`
- Note: Aborts the transaction, discards all queued commands.

### FR-8: WATCH key [key ...]
- Method: `watch(&self, keys: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `WATCH key1 [key2 ...]`
- Unit test: `test_command_watch_encoding`
- Note: Monitors keys for changes before a transaction.

### FR-9: UNWATCH
- Method: `unwatch(&self) -> CommandBuilder`
- RESP: `UNWATCH`
- Unit test: `test_command_unwatch_encoding`
- Note: Clears all watched keys.

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- Pub/sub and transaction commands are wire-format only — they do not need InMemoryClient support since they change connection state

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (after `append` method)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests`

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 9 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
