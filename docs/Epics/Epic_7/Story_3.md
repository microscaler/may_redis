# Story 7.3 — Set Commands

**Objective:** Add set operations beyond SADD/SREM/SISMEMBER. These include enumeration, set algebra (intersection, union, difference), element removal, and cursor-based iteration.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for set commands)

## Struct

This story adds the following methods to `Commands`:

```rust
pub trait Commands: Sized {
    // ... existing 22 methods ...

    // NEW: Set Commands
    fn smembers<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn spop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn spop_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder;
    fn srandmember<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn srandmember_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder;
    fn scard<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn sinter(keys: &[impl ToRedisArgs]) -> CommandBuilder;
    fn sunion(keys: &[impl ToRedisArgs]) -> CommandBuilder;
    fn smove<K: ToRedisArgs, M: ToRedisArgs>(&self, source: K, destination: K, member: M) -> CommandBuilder;
    fn sscan<K: ToRedisArgs>(&self, key: K, cursor: i64) -> CommandBuilder;
    fn sscan_match<K: ToRedisArgs>(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder;
}
```

## Implementation Pattern

Same as SADD/SISMEMBER. For variadic variants (SINTER, SUNION), use `args()`:

```rust
fn sinter(keys: &[impl ToRedisArgs]) -> CommandBuilder {
    let mut builder = CommandBuilder::new("SINTER");
    for key in keys {
        builder = builder.arg(key);
    }
    builder
}
```

## Tasks

- [ ] Define `smembers(key)` → `cmd("SMEMBERS").arg(key)`
- [ ] Define `spop(key)` → `cmd("SPOP").arg(key)`
- [ ] Define `spop_count(key, count)` → `cmd("SPOP").arg(key).arg(count)`
- [ ] Define `srandmember(key)` → `cmd("SRANDMEMBER").arg(key)`
- [ ] Define `srandmember_count(key, count)` → `cmd("SRANDMEMBER").arg(key).arg(count)`
- [ ] Define `scard(key)` → `cmd("SCARD").arg(key)`
- [ ] Define `sinter(keys)` → `cmd("SINTER").args(keys)`
- [ ] Define `sunion(keys)` → `cmd("SUNION").args(keys)`
- [ ] Define `smove(source, dest, member)` → `cmd("SMOVE").arg(source).arg(dest).arg(member)`
- [ ] Define `sscan(key, cursor)` → `cmd("SSCAN").arg(key).arg(cursor)`
- [ ] Define `sscan_match(key, cursor, pattern)` → `cmd("SSCAN").arg(key).arg(cursor).arg("MATCH").arg(pattern)`
- [ ] Add unit test for each method in `mod tests`

## Verification

- `cargo check --lib` passes
- `cargo test --lib test_command_smembers_encoding` — `cmd("SMEMBERS").arg("s").build()` → correct bytes
- `cargo test --lib test_command_spop_encoding` — `cmd("SPOP").arg("s").build()` → correct bytes
- `cargo test --lib test_command_spop_count_encoding` — `cmd("SPOP").arg("s").arg(3).build()` → correct bytes
- `cargo test --lib test_command_srandmember_encoding` — `cmd("SRANDMEMBER").arg("s").build()` → correct bytes
- `cargo test --lib test_command_srandmember_count_encoding` — `cmd("SRANDMEMBER").arg("s").arg(2).build()` → correct bytes
- `cargo test --lib test_command_scard_encoding` — `cmd("SCARD").arg("s").build()` → correct bytes
- `cargo test --lib test_command_sinter_encoding` — `cmd("SINTER").args(&["s1","s2"]).build()` → correct bytes
- `cargo test --lib test_command_sunion_encoding` — `cmd("SUNION").args(&["s1","s2"]).build()` → correct bytes
- `cargo test --lib test_command_smove_encoding` — `cmd("SMOVE").arg("src").arg("dst").arg("m").build()` → correct bytes
- `cargo test --lib test_command_sscan_encoding` — `cmd("SSCAN").arg("s").arg(0).build()` → correct bytes
- `cargo test --lib test_command_sscan_match_encoding` — `cmd("SSCAN").arg("s").arg(0).arg("MATCH").arg("*").build()` → correct bytes
- `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
