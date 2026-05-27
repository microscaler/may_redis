# Story 3.2 — Commands trait

**Objective:** Implement the `Commands` trait with methods for every Redis command used by Sesame-IDAM.

**Epic:** 3 — Protocol Crate

**Dependencies:** Story 3.1

**Source docs:** `docs/07-client-api-design.md`, `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `crates/protocol/src/lib.rs` — `pub trait Commands`
- `crates/protocol/src/commands.rs` — trait impls

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

1. Define `Commands` trait with all 14 methods listed above
2. Implement `get(key)` → `cmd("GET").arg(key)`
3. Implement `set(key, value)` → `cmd("SET").arg(key).arg(value)`
4. Implement `set_ex(key, value, seconds)` → `cmd("SET").arg(key).arg(value).arg("EX").arg(seconds)`
5. Implement `exists(key)` → `cmd("EXISTS").arg(key)`
6. Implement `del(key)` → `cmd("DEL").arg(key)`
7. Implement `incr(key)` → `cmd("INCR").arg(key)`
8. Implement `ttl(key)` → `cmd("TTL").arg(key)`
9. Implement `expire(key, seconds)` → `cmd("EXPIRE").arg(key).arg(seconds)`
10. Implement `publish(channel, message)` → `cmd("PUBLISH").arg(channel).arg(message)`
11. Implement `keys(pattern)` → `cmd("KEYS").arg(pattern)`
12. Implement `dbsize()` → `cmd("DBSIZE")`
13. Implement `flushdb()` → `cmd("FLUSHDB")`
14. Implement `ping()` → `cmd("PING")`
15. Implement `auth(password)` → `cmd("AUTH").arg(password)`

## Verification

- `cargo test -p protocol` — at least 14 unit tests (one per method):
  - Each test verifies the encoded BytesMut matches expected RESP format
  - `test_command_get_encoding` — GET key → `*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n`
  - `test_command_set_encoding` — SET key val → `*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n`
  - `test_command_set_ex_encoding` — SET key val EX 60 → correct bytes
  - `test_command_exists_encoding` — EXISTS key → correct bytes
  - `test_command_del_encoding` — DEL key → correct bytes
  - `test_command_incr_encoding` — INCR key → correct bytes
  - `test_command_ttl_encoding` — TTL key → correct bytes
  - `test_command_expire_encoding` — EXPIRE key 60 → correct bytes
  - `test_command_publish_encoding` — PUBLISH ch msg → correct bytes
  - `test_command_keys_encoding` — KEYS pat → correct bytes
  - `test_command_dbsize_encoding` — DBSIZE → `*1\r\n$6\r\nDBSIZE\r\n`
  - `test_command_flushdb_encoding` — FLUSHDB → `*1\r\n$7\r\nFLUSHDB\r\n`
  - `test_command_ping_encoding` — PING → `*1\r\n$4\r\nPING\r\n`
  - `test_command_auth_encoding` — AUTH pass → correct bytes
- `cargo clippy -p protocol` — zero warnings
