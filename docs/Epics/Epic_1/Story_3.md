# Story 1.3 — ToRedisArgs trait

**Objective:** Implement the `ToRedisArgs` trait for converting Rust types to Redis command arguments.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.2

**Source docs:** `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `crates/base/src/to_redis_args.rs` — `pub trait ToRedisArgs` + impls

## Struct

```rust
pub trait ToRedisArgs {
    fn write_redis_args(&self, buf: &mut Vec<u8>);
    fn is_simple_arg(&self) -> bool;
}
```

## Tasks

1. Define `ToRedisArgs` trait with `write_redis_args` and `is_simple_arg` methods
2. Implement `ToRedisArgs` for `String` — writes raw bytes
3. Implement `ToRedisArgs` for `&str` — writes as bulk string
4. Implement `ToRedisArgs` for `i64` — writes decimal representation
5. Implement `ToRedisArgs` for `u32` — writes decimal representation
6. Implement `ToRedisArgs` for `&[u8]` — writes raw bytes
7. Implement `ToRedisArgs` for `Vec<String>` — writes each element as bulk string
8. Implement `ToRedisArgs` for `&[String]` — writes each element as bulk string

## Verification

- `cargo test -p base` — at least 6 unit tests:
  - `test_to_redis_args_string` — "SET" → writes "SET" bytes
  - `test_to_redis_args_i64` — 42i64 → writes "42" bytes
  - `test_to_redis_args_u32` — 60u32 → writes "60" bytes
  - `test_to_redis_args_str` — &str writes same as String
  - `test_to_redis_args_bytes` — &[u8] writes raw bytes
  - `test_to_redis_args_vec_string` — vec!["A","B"] writes both
- `cargo clippy -p base` — zero warnings
