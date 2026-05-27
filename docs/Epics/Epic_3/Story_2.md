# Story 3.2 — Commands trait

**Objective:** Implement the `Commands` trait with methods for every Redis command used by Sesame-IDAM.

**Epic:** 3 — Protocol Crate

**Dependencies:** Story 3.1

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/07-client-api-design.md`, `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `src/protocol/mod.rs` — `pub trait Commands`
- `src/protocol/commands.rs` — trait impls

## Struct

```rust
pub trait Commands: Sized {
    fn get<K: ToRedisArgs, V: FromRedisValue>(&self, key: K) -> CommandBuilder;
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder;
    fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V, seconds: u32) -> CommandBuilder;
    fn exists<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn del<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn incr<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn ttl<K: ToRedisArgs>(&self, key: K) -> CommandBuilder;
    fn expire<K: ToRedisArgs>(&self, key: K, seconds: u32) -> CommandBuilder;
    fn publish<K: ToRedisArgs, M: ToRedisArgs>(&self, channel: K, message: M) -> CommandBuilder;
    fn keys<K: ToRedisArgs>(&self, pattern: K) -> CommandBuilder;
    fn dbsize(&self) -> CommandBuilder;
    fn flushdb(&self) -> CommandBuilder;
    fn ping(&self) -> CommandBuilder;
    fn auth(&self, password: &str) -> CommandBuilder;
}
```

## Tasks

- [x] Define `Commands` trait with all 14 methods listed above
- [x] Implement `get(key)` → `cmd("GET").arg(key)`
- [x] Implement `set(key, value)` → `cmd("SET").arg(key).arg(value)`
- [x] Implement `set_ex(key, value, seconds)` → `cmd("SET").arg(key).arg(value).arg("EX").arg(seconds)`
- [x] Implement `exists(key)` → `cmd("EXISTS").arg(key)`
- [x] Implement `del(key)` → `cmd("DEL").arg(key)`
- [x] Implement `incr(key)` → `cmd("INCR").arg(key)`
- [x] Implement `ttl(key)` → `cmd("TTL").arg(key)`
- [x] Implement `expire(key, seconds)` → `cmd("EXPIRE").arg(key).arg(seconds)`
- [x] Implement `publish(channel, message)` → `cmd("PUBLISH").arg(channel).arg(message)`
- [x] Implement `keys(pattern)` → `cmd("KEYS").arg(pattern)`
- [x] Implement `dbsize()` → `cmd("DBSIZE")`
- [x] Implement `flushdb()` → `cmd("FLUSHDB")`
- [x] Implement `ping()` → `cmd("PING")`
- [x] Implement `auth(password)` → `cmd("AUTH").arg(password)`

## Verification

- All 14 command encoding tests pass:
  - `test_command_get_encoding` — GET key → correct bytes
  - `test_command_set_encoding` — SET key val → correct bytes
  - `test_command_set_ex_encoding` — SET key val EX 60 → correct bytes
  - `test_command_exists_encoding` — EXISTS key → correct bytes
  - `test_command_del_encoding` — DEL key → correct bytes
  - `test_command_incr_encoding` — INCR key → correct bytes
  - `test_command_ttl_encoding` — TTL key → correct bytes
  - `test_command_expire_encoding` — EXPIRE key 60 → correct bytes
  - `test_command_publish_encoding` — PUBLISH ch msg → correct bytes
  - `test_command_keys_encoding` — KEYS pat → correct bytes
  - `test_command_dbsize_encoding` — DBSIZE → correct bytes
  - `test_command_flushdb_encoding` — FLUSHDB → correct bytes
  - `test_command_ping_encoding` — PING → correct bytes
  - `test_command_auth_encoding` — AUTH pass → correct bytes
- `cargo clippy` — zero warnings
