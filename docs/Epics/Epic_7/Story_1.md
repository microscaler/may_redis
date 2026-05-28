# Story 7.1 — String Extension Commands

**Objective:** Add string extension commands that complement the existing SET/GET/DEL base. These include DEPR, MGET, MSET, SETNX, bulk ops, bit operations, and range ops.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for string commands)

## Functional Requirements

### FR-1: DECR key
- Method: `decr(&self, key: K) -> CommandBuilder`
- RESP: `DECR key`
- Unit test: `test_command_decr_encoding`
- Wire format: `*2\r\n$4\r\nDECR\r\n$N\r\n<key>\r\n`

### FR-2: DECRBY key decrement
- Method: `decrby(&self, key: K, decrement: i64) -> CommandBuilder`
- RESP: `DECRBY key decrement`
- Unit test: `test_command_decrby_encoding`
- Wire format: `*3\r\n$6\r\nDECRBY\r\n$N\r\n<key>\r\n$M\r\n<decrement>\r\n`

### FR-3: SETNX key value
- Method: `setnx(&self, key: K, value: V) -> CommandBuilder`
- RESP: `SETNX key value`
- Unit test: `test_command_setnx_encoding`
- Wire format: `*3\r\n$5\r\nSETNX\r\n$N\r\n<key>\r\n$M\r\n<value>\r\n`

### FR-4: MGET key...
- Method: `mget(&self, keys: &[impl ToRedisArgs]) -> CommandBuilder`
- RESP: `MGET key1 key2 ...`
- Unit test: `test_command_mget_encoding`
- Wire format: `*<n+1>\r\n$4\r\nMGET\r\n$N\r\n<key1>\r\n$N\r\n<key2>\r\n...`
- Uses `args()` helper on CommandBuilder for variadic keys

### FR-5: MSET key value key value...
- Method: `mset(&self, pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder`
- RESP: `MSET key1 value1 key2 value2 ...`
- Unit test: `test_command_mset_encoding`
- Wire format: `*<n*2+1>\r\n$4\r\nMSET\r\n$N\r\n<key1>\r\n$M\r\n<value1>\r\n...`

### FR-6: MSETNX key value key value...
- Method: `msetnx(&self, pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder`
- RESP: `MSETNX key1 value1 key2 value2 ...`
- Unit test: `test_command_msetnx_encoding`
- Wire format: Same as MSET but with MSETNX command name

### FR-7: STRLEN key
- Method: `strlen(&self, key: K) -> CommandBuilder`
- RESP: `STRLEN key`
- Unit test: `test_command_strlen_encoding`

### FR-8: GETRANGE key start end
- Method: `getrange(&self, key: K, start: i64, end: i64) -> CommandBuilder`
- RESP: `GETRANGE key start end`
- Unit test: `test_command_getrange_encoding`

### FR-9: SETRANGE key offset value
- Method: `setrange(&self, key: K, offset: i64, value: V) -> CommandBuilder`
- RESP: `SETRANGE key offset value`
- Unit test: `test_command_setrange_encoding`

### FR-10: SETBIT key offset value
- Method: `setbit(&self, key: K, offset: i64, value: i64) -> CommandBuilder`
- RESP: `SETBIT key offset value`
- Unit test: `test_command_setbit_encoding`

### FR-11: GETBIT key offset
- Method: `getbit(&self, key: K, offset: i64) -> CommandBuilder`
- RESP: `GETBIT key offset`
- Unit test: `test_command_getbit_encoding`

### FR-12: BITCOUNT key [start end]
- Method: `bitcount(&self, key: K) -> CommandBuilder` (simple form)
- Method: `bitcount_range(&self, key: K, start: i64, end: i64) -> CommandBuilder` (with range)
- RESP: `BITCOUNT key` / `BITCOUNT key start end`
- Unit tests: `test_command_bitcount_encoding`, `test_command_bitcount_range_encoding`

## Non-Functional Requirements

- All methods follow existing pattern: `#[must_use]`, `CommandBuilder::new()`, `.arg()`
- No new dependencies in Cargo.toml
- Every method has exactly one `test_command_*_encoding` unit test in `mod tests`
- Methods that take variadic args use `CommandBuilder::args()` for the variadic portion
- All new tests compile and run with `#[test]` (no runtime needed)

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (~lines 160-340)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests`

## Verification

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib test_command_` passes with zero failures
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` passes
- [ ] All 12 new methods have unit tests
- [ ] Wire encoding for each command matches RESP2 spec
