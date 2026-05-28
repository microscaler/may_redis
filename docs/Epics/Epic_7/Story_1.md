# Story 7.1 — String Extension Commands

**Objective:** Add string extension commands that complement the existing SET/GET/DEL base. These cover decrement variants, bulk operations, set-if-not-exists, bit operations, and range operations.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for string commands)

## Code Anchors

- `src/protocol/commands.rs` — Add methods to `Commands` trait (after line 164, after `append`)
- `src/protocol/commands.rs::tests` — Add test functions at end of `mod tests` block

## Struct

The `Commands` trait currently has 22 methods. This story adds the following:

```rust
pub trait Commands: Sized {
    // ... existing 22 methods ...

    // NEW: String Extension Commands
    fn decr<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn decrby<K: ToRedisArgs>(&self, key: K, decrement: i64) -> CommandBuilder;
    fn setnx<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder;
    fn mget(keys: &[impl ToRedisArgs]) -> CommandBuilder;
    fn mset(pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder;
    fn msetnx(pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder;
    fn strlen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn getrange<K: ToRedisArgs>(&self, key: K, start: i64, end: i64) -> CommandBuilder;
    fn setrange<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, offset: i64, value: V) -> CommandBuilder;
    fn setbit<K: ToRedisArgs>(&self, key: K, offset: i64, value: i64) -> CommandBuilder;
    fn getbit<K: ToRedisArgs>(&self, key: K, offset: i64) -> CommandBuilder;
    fn bitcount<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn bitcount_range<K: ToRedisArgs>(&self, key: K, start: i64, end: i64) -> CommandBuilder;
}
```

## Implementation Pattern

Each method follows the established pattern:

```rust
/// DESCRIPTION
#[must_use = "call .build() to encode the command"]
fn command_name<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
    CommandBuilder::new("COMMAND_NAME").arg(key)
}
```

For variadic args (MGET, MSET, MSETNX), use `CommandBuilder::args()`:

```rust
fn mget(keys: &[impl ToRedisArgs]) -> CommandBuilder {
    let mut builder = CommandBuilder::new("MGET");
    for key in keys {
        builder = builder.arg(key);
    }
    builder
}
```

## Tasks

- [ ] Define `decr(key)` → `cmd("DECR").arg(key)`
- [ ] Define `decrby(key, decrement)` → `cmd("DECRBY").arg(key).arg(decrement)`
- [ ] Define `setnx(key, value)` → `cmd("SETNX").arg(key).arg(value)`
- [ ] Define `mget(keys)` → `cmd("MGET").args(keys)` — variadic key list
- [ ] Define `mset(pairs)` → `cmd("MSET").args(pairs)` — variadic key-value list
- [ ] Define `msetnx(pairs)` → `cmd("MSETNX").args(pairs)` — variadic key-value list
- [ ] Define `strlen(key)` → `cmd("STRLEN").arg(key)`
- [ ] Define `getrange(key, start, end)` → `cmd("GETRANGE").arg(key).arg(start).arg(end)`
- [ ] Define `setrange(key, offset, value)` → `cmd("SETRANGE").arg(key).arg(offset).arg(value)`
- [ ] Define `setbit(key, offset, value)` → `cmd("SETBIT").arg(key).arg(offset).arg(value)`
- [ ] Define `getbit(key, offset)` → `cmd("GETBIT").arg(key).arg(offset)`
- [ ] Define `bitcount(key)` → `cmd("BITCOUNT").arg(key)`
- [ ] Define `bitcount_range(key, start, end)` → `cmd("BITCOUNT").arg(key).arg(start).arg(end)`
- [ ] Add unit test for each method in `mod tests`

## Verification

- `cargo check --lib` passes
- `cargo test --lib test_command_decr_encoding` — `cmd("DECR").arg("k").build()` → `*2\r\n$4\r\nDECR\r\n$1\r\nk\r\n`
- `cargo test --lib test_command_decrby_encoding` — `cmd("DECRBY").arg("k").arg(5).build()` → correct bytes
- `cargo test --lib test_command_setnx_encoding` — `cmd("SETNX").arg("k").arg("v").build()` → correct bytes
- `cargo test --lib test_command_mget_encoding` — `mget(&["k1","k2"])` → `*3\r\n$4\r\nMGET\r\n$2\r\nk1\r\n$2\r\nk2\r\n`
- `cargo test --lib test_command_mset_encoding` — `mset(&[("k1","v1")])` → correct bytes
- `cargo test --lib test_command_msetnx_encoding` — `msetnx(&[("k1","v1")])` → correct bytes
- `cargo test --lib test_command_strlen_encoding` — correct bytes
- `cargo test --lib test_command_getrange_encoding` — correct bytes
- `cargo test --lib test_command_setrange_encoding` — correct bytes
- `cargo test --lib test_command_setbit_encoding` — correct bytes
- `cargo test --lib test_command_getbit_encoding` — correct bytes
- `cargo test --lib test_command_bitcount_encoding` — correct bytes
- `cargo test --lib test_command_bitcount_range_encoding` — correct bytes
- `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
