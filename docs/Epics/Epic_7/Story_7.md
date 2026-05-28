# Story 7.7 — Server/Admin Commands

**Objective:** Add server administration and utility commands. These cover database selection, key type introspection, key management (move/rename/sort), cursor-based iteration (SCAN), TTL variants (PTTL, PEXPIRE, PERSIST), persistence commands (SAVE, BGSAVE), and shutdown.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup), Story 7.6 (Pub/Sub + Transactions — runs in parallel)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for server commands), `docs/08-command-audit.md` (server/admin coverage)

## Functional Requirements

### FR-1: SELECT index
- Method: `select(&self, index: i64) -> CommandBuilder`
- RESP: `SELECT index`
- Unit test: `test_command_select_encoding`

### FR-2: TYPE key
- Method: `type(&self, key: K) -> CommandBuilder`
- RESP: `TYPE key`
- Unit test: `test_command_type_encoding`

### FR-3: MOVE key db
- Method: `move_key(&self, key: K, db: i64) -> CommandBuilder`
- RESP: `MOVE key db`
- Unit test: `test_command_move_encoding`

### FR-4: RENAME key newkey
- Method: `rename(&self, key: K, newkey: K) -> CommandBuilder`
- RESP: `RENAME key newkey`
- Unit test: `test_command_rename_encoding`

### FR-5: RENAMENX key newkey
- Method: `renamemx(&self, key: K, newkey: K) -> CommandBuilder`
- RESP: `RENAMENX key newkey`
- Unit test: `test_command_renamemx_encoding`

### FR-6: SORT key [BY pattern] [LIMIT offset count] [GET pattern ...] [ASC|DESC] [ALPHA] [STORE dest]
- Method: `sort(&self, key: K) -> CommandBuilder` (simple form)
- Method: `sort_limit(&self, key: K, offset: i64, count: i64) -> CommandBuilder`
- Method: `sort_limit_order(&self, key: K, offset: i64, count: i64, order: &str) -> CommandBuilder`
- RESP: `SORT key` / `SORT key LIMIT offset count` / `SORT key LIMIT offset count ASC/DESC`
- Unit tests: `test_command_sort_encoding`, `test_command_sort_limit_encoding`, `test_command_sort_limit_order_encoding`

### FR-7: SCAN cursor [MATCH pattern] [COUNT count]
- Method: `scan(&self, cursor: i64) -> CommandBuilder` (simple form)
- Method: `scan_match(&self, cursor: i64, pattern: &str) -> CommandBuilder` (with match)
- RESP: `SCAN cursor` / `SCAN cursor MATCH pattern [COUNT count]`
- Unit tests: `test_command_scan_encoding`, `test_command_scan_match_encoding`

### FR-8: TOUCH key [key ...]
- Method: `touch(&self, keys: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `TOUCH key1 [key2 ...]`
- Unit test: `test_command_touch_encoding`

### FR-9: SAVE
- Method: `save(&self) -> CommandBuilder`
- RESP: `SAVE`
- Unit test: `test_command_save_encoding`

### FR-10: BGSAVE
- Method: `bgsave(&self) -> CommandBuilder`
- RESP: `BGSAVE`
- Unit test: `test_command_bgsave_encoding`

### FR-11: FLUSHALL
- Method: `flushall(&self) -> CommandBuilder`
- RESP: `FLUSHALL`
- Unit test: `test_command_flushall_encoding`

### FR-12: PTTL key
- Method: `pttl(&self, key: K) -> CommandBuilder`
- RESP: `PTTL key`
- Unit test: `test_command_pttl_encoding`

### FR-13: PEXPIRE key milliseconds
- Method: `pexpire(&self, key: K, ms: i64) -> CommandBuilder`
- RESP: `PEXPIRE key milliseconds`
- Unit test: `test_command_pexpire_encoding`

### FR-14: PEXPIREAT key timestamp-ms
- Method: `pexpireat(&self, key: K, timestamp_ms: i64) -> CommandBuilder`
- RESP: `PEXPIREAT key timestamp`
- Unit test: `test_command_pexpireat_encoding`

### FR-15: PERSIST key
- Method: `persist(&self, key: K) -> CommandBuilder`
- RESP: `PERSIST key`
- Unit test: `test_command_persist_encoding`

### FR-16: SHUTDOWN [NOSAVE|SAVE] [NOSAVE|SAVE] [FORCE] [NOW] [KILL]
- Method: `shutdown(&self) -> CommandBuilder` (simple form)
- Method: `shutdown_nosave(&self) -> CommandBuilder` (without save)
- RESP: `SHUTDOWN` / `SHUTDOWN NOSAVE`
- Unit tests: `test_command_shutdown_encoding`, `test_command_shutdown_nosave_encoding`

### FR-17: INFO [section]
- Method: `info(&self) -> CommandBuilder` (all sections)
- Method: `info_server(&self) -> CommandBuilder` (server section)
- RESP: `INFO` / `INFO server`
- Unit tests: `test_command_info_encoding`, `test_command_info_server_encoding`

### FR-18: CONFIG GET parameter
- Method: `config_get(&self, parameter: &str) -> CommandBuilder`
- RESP: `CONFIG GET parameter`
- Unit test: `test_command_config_get_encoding`

## Non-Functional Requirements

- Same `#[must_use]`, `CommandBuilder::new()` pattern as existing commands
- No new dependencies
- Every method has exactly one `test_command_*_encoding` unit test
- Commands with variadic keys (TOUCH, SORT with multiple GET patterns) use `CommandBuilder::args()`
- Commands with optional flags (SHUTDOWN, SORT, SCAN, INFO) have simple and extended variants

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 18 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
