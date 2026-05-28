# Story 7.4 — List Commands

**Objective:** Add list manipulation commands. Lists cover push/pop at both ends, range queries, element access, modification, and blocking operations.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for list commands)

## Struct

This story adds the following methods to `Commands`:

```rust
pub trait Commands: Sized {
    // ... existing 22 methods ...

    // NEW: List Commands
    fn lpush<K: ToRedisArgs>(&self, key: K, values: &[impl ToRedisArgs]) -> CommandBuilder;
    fn rpush<K: ToRedisArgs>(&self, key: K, values: &[impl ToRedisArgs]) -> CommandBuilder;
    fn lpop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn rpop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn llen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn lrange<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder;
    fn lindex<K: ToRedisArgs>(&self, key: K, index: i64) -> CommandBuilder;
    fn lset<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, index: i64, value: V) -> CommandBuilder;
    fn lrem<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, count: i64, value: V) -> CommandBuilder;
    fn ltrim<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder;
    fn blpop<K: ToRedisArgs>(&self, keys: &[impl ToRedisArgs], timeout: i64) -> CommandBuilder;
    fn brpop<K: ToRedisArgs>(&self, keys: &[impl ToRedisArgs], timeout: i64) -> CommandBuilder;
}
```

## Implementation Pattern

For variadic variants (LPUSH, RPUSH, BLPOP, BRPOP), use `args()`:

```rust
fn lpush<K: ToRedisArgs>(&self, key: K, values: &[impl ToRedisArgs]) -> CommandBuilder {
    let mut builder = CommandBuilder::new("LPUSH").arg(key);
    for v in values {
        builder = builder.arg(v);
    }
    builder
}
```

## Tasks

- [ ] Define `lpush(key, values)` → `cmd("LPUSH").arg(key).args(values)`
- [ ] Define `rpush(key, values)` → `cmd("RPUSH").arg(key).args(values)`
- [ ] Define `lpop(key)` → `cmd("LPOP").arg(key)`
- [ ] Define `rpop(key)` → `cmd("RPOP").arg(key)`
- [ ] Define `llen(key)` → `cmd("LLEN").arg(key)`
- [ ] Define `lrange(key, start, stop)` → `cmd("LRANGE").arg(key).arg(start).arg(stop)`
- [ ] Define `lindex(key, index)` → `cmd("LINDEX").arg(key).arg(index)`
- [ ] Define `lset(key, index, value)` → `cmd("LSET").arg(key).arg(index).arg(value)`
- [ ] Define `lrem(key, count, value)` → `cmd("LREM").arg(key).arg(count).arg(value)`
- [ ] Define `ltrim(key, start, stop)` → `cmd("LTRIM").arg(key).arg(start).arg(stop)`
- [ ] Define `blpop(keys, timeout)` → `cmd("BLPOP").args(keys).arg(timeout)`
- [ ] Define `brpop(keys, timeout)` → `cmd("BRPOP").args(keys).arg(timeout)`
- [ ] Add unit test for each method in `mod tests`

## Verification

- `cargo check --lib` passes
- `cargo test --lib test_command_lpush_encoding` — `cmd("LPUSH").arg("l").args(&["v1","v2"]).build()` → correct bytes
- `cargo test --lib test_command_rpush_encoding` — `cmd("RPUSH").arg("l").args(&["v1"]).build()` → correct bytes
- `cargo test --lib test_command_lpop_encoding` — `cmd("LPOP").arg("l").build()` → correct bytes
- `cargo test --lib test_command_rpop_encoding` — `cmd("RPOP").arg("l").build()` → correct bytes
- `cargo test --lib test_command_llen_encoding` — `cmd("LLEN").arg("l").build()` → correct bytes
- `cargo test --lib test_command_lrange_encoding` — `cmd("LRANGE").arg("l").arg(0).arg(-1).build()` → correct bytes
- `cargo test --lib test_command_lindex_encoding` — `cmd("LINDEX").arg("l").arg(0).build()` → correct bytes
- `cargo test --lib test_command_lset_encoding` — `cmd("LSET").arg("l").arg(0).arg("v").build()` → correct bytes
- `cargo test --lib test_command_lrem_encoding` — `cmd("LREM").arg("l").arg(0).arg("v").build()` → correct bytes
- `cargo test --lib test_command_ltrim_encoding` — `cmd("LTRIM").arg("l").arg(0).arg(10).build()` → correct bytes
- `cargo test --lib test_command_blpop_encoding` — `cmd("BLPOP").args(&["l"]).arg(0).build()` → correct bytes
- `cargo test --lib test_command_brpop_encoding` — `cmd("BRPOP").args(&["l"]).arg(0).build()` → correct bytes
- `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
