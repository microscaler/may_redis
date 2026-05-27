# Story 3.1 — CommandBuilder

**Objective:** Implement the fluent `CommandBuilder` for building Redis commands.

**Epic:** 3 — Protocol Crate

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec)

**Source docs:** `docs/07-client-api-design.md`, `docs/Epics/Epic_3/Story_0.md`

## Code Anchors

- `crates/protocol/src/lib.rs` — `pub struct CommandBuilder { args: Vec<RedisValue> }`
- `crates/protocol/src/builder.rs` — implementation

## Struct

```rust
pub struct CommandBuilder {
    args: Vec<RedisValue>,
}
```

## Methods

```rust
impl CommandBuilder {
    pub fn new(cmd: &str) -> Self;
    pub fn arg<V: ToRedisArgs>(mut self, val: V) -> Self;
    pub fn args<V: ToRedisArgs>(mut self, vals: &[V]) -> Self;
    pub fn build(self) -> BytesMut;
}

pub fn cmd(cmd: &str) -> CommandBuilder;
```

## Tasks

1. Define `CommandBuilder` with `args: Vec<RedisValue>`
2. Implement `new(cmd)` — converts command name to RedisValue::BulkString
3. Implement `arg(val)` — converts via ToRedisArgs → RedisValue, appends to args
4. Implement `args(vals)` — batch append multiple args
5. Implement `build()` — uses codec crate's RESPWriter to encode args into BytesMut
6. Implement `cmd()` convenience function — creates CommandBuilder and calls new()
7. Add `len()` method returning number of arguments (useful for testing)

## Verification

- `cargo test -p protocol` — at least 5 unit tests:
  - `test_cmd_set_key_value` — cmd("SET").arg("k").arg("v").build() → correct RESP bytes
  - `test_cmd_get_key` — cmd("GET").arg("key").build() → correct RESP bytes
  - `test_cmd_with_multiple_args` — cmd("MSET").args(&["k1","v1","k2","v2"]) → correct bytes
  - `test_cmd_len` — cmd("PING").len() == 1
  - `test_cmd_len_with_args` — cmd("SET").arg("k").arg("v").len() == 3
- `cargo clippy -p protocol` — zero warnings
- `cargo doc -p protocol` — all public items documented
