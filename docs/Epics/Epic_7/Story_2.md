# Story 7.2 — Hash Commands

**Objective:** Add hash manipulation commands beyond HSET/HGET. These cover field deletion, key enumeration, full hash retrieval, multi-field operations, and cursor-based iteration.

**Epic:** 7 — Redis Command Expansion

**Dependencies:** Epic 7.0 (epic setup)

**Status:** PENDING

**Source docs:** `docs/01-protocol-analysis.md` (RESP encoding for hash commands)

## Struct

This story adds the following methods to `Commands`:

```rust
pub trait Commands: Sized {
    // ... existing 22 methods ...

    // NEW: Hash Commands
    fn hdel<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder;
    fn hdel_fields<K: ToRedisArgs>(&self, key: K, fields: &[impl ToRedisArgs]) -> CommandBuilder;
    fn hkeys<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn hgetall<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn hmset<K: ToRedisArgs>(&self, key: K, pairs: &[(impl ToRedisArgs, impl ToRedisArgs)]) -> CommandBuilder;
    fn hincrby<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F, increment: i64) -> CommandBuilder;
    fn hlen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn hexists<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder;
    fn hscan<K: ToRedisArgs>(&self, key: K, cursor: i64) -> CommandBuilder;
    fn hscan_match<K: ToRedisArgs>(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder;
}
```

## Implementation Pattern

Same pattern as HSET/HGET. For variadic variants (HDEL fields, HMSET), use `args()`:

```rust
fn hdel_fields<K: ToRedisArgs>(&self, key: K, fields: &[impl ToRedisArgs]) -> CommandBuilder {
    let mut builder = CommandBuilder::new("HDEL").arg(key);
    for f in fields {
        builder = builder.arg(f);
    }
    builder
}
```

## Tasks

- [ ] Define `hdel(key, field)` → `cmd("HDEL").arg(key).arg(field)`
- [ ] Define `hdel_fields(key, fields)` → `cmd("HDEL").arg(key).args(fields)`
- [ ] Define `hkeys(key)` → `cmd("HKEYS").arg(key)`
- [ ] Define `hgetall(key)` → `cmd("HGETALL").arg(key)`
- [ ] Define `hmset(key, pairs)` → `cmd("HMSET").arg(key).args(pairs)`
- [ ] Define `hincrby(key, field, increment)` → `cmd("HINCRBY").arg(key).arg(field).arg(increment)`
- [ ] Define `hlen(key)` → `cmd("HLEN").arg(key)`
- [ ] Define `hexists(key, field)` → `cmd("HEXISTS").arg(key).arg(field)`
- [ ] Define `hscan(key, cursor)` → `cmd("HSCAN").arg(key).arg(cursor)`
- [ ] Define `hscan_match(key, cursor, pattern)` → `cmd("HSCAN").arg(key).arg(cursor).arg("MATCH").arg(pattern)`
- [ ] Add unit test for each method in `mod tests`

## Verification

- `cargo check --lib` passes
- `cargo test --lib test_command_hdel_encoding` — `cmd("HDEL").arg("h").arg("f").build()` → correct bytes
- `cargo test --lib test_command_hkeys_encoding` — `cmd("HKEYS").arg("h").build()` → correct bytes
- `cargo test --lib test_command_hgetall_encoding` — `cmd("HGETALL").arg("h").build()` → correct bytes
- `cargo test --lib test_command_hmset_encoding` — `cmd("HMSET").args(&["h","f","v"])` → correct bytes
- `cargo test --lib test_command_hincrby_encoding` — `cmd("HINCRBY").arg("h").arg("f").arg(1).build()` → correct bytes
- `cargo test --lib test_command_hlen_encoding` — `cmd("HLEN").arg("h").build()` → correct bytes
- `cargo test --lib test_command_hexists_encoding` — `cmd("HEXISTS").arg("h").arg("f").build()` → correct bytes
- `cargo test --lib test_command_hscan_encoding` — `cmd("HSCAN").arg("h").arg(0).build()` → correct bytes
- `cargo test --lib test_command_hscan_match_encoding` — `cmd("HSCAN").arg("h").arg(0).arg("MATCH").arg("*").build()` → correct bytes
- `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
