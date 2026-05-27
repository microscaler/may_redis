# Story 1.3 — ToRedisArgs trait

**Objective:** Implement the `ToRedisArgs` trait for converting Rust types to Redis command arguments.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.2

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `src/core/to_args.rs` — `pub trait ToRedisArgs` + impls

## Struct

```rust
pub trait ToRedisArgs {
    fn write_redis_args(&self, buf: &mut Vec<u8>);
    fn is_simple_arg(&self) -> bool;
}
```

## Tasks

- [x] Define `ToRedisArgs` trait with `write_redis_args` and `is_simple_arg` methods
- [x] Implement `ToRedisArgs` for `String` — writes raw bytes
- [x] Implement `ToRedisArgs` for `&str` — writes as bulk string
- [x] Implement `ToRedisArgs` for `i64` — writes decimal representation
- [x] Implement `ToRedisArgs` for `u32` — writes decimal representation
- [x] Implement `ToRedisArgs` for `&[u8]` — writes raw bytes
- [x] Implement `ToRedisArgs` for `Vec<String>` — writes each element as bulk string
- [x] Implement `ToRedisArgs` for `&[String]` — writes each element as bulk string

## Verification

- All tests pass:
  - `test_to_redis_args_string`
  - `test_to_redis_args_i64`
  - `test_to_redis_args_u32`
  - `test_to_redis_args_str`
  - `test_to_redis_args_bytes`
  - `test_to_redis_args_vec_string`
- `cargo clippy` — zero warnings
