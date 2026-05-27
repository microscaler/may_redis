# Story 1.1 — RedisValue enum

**Objective:** Implement the `RedisValue` enum representing all Redis data types. This is the single most important type in the crate.

**Epic:** 1 — Base Crate

**Dependencies:** Epic 0 (scaffolding)

**Source docs:** `docs/01-protocol-analysis.md`, `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `crates/base/src/lib.rs` — `pub enum RedisValue { ... }`
- `crates/base/src/redis_value.rs` — `impl RedisValue` blocks

## Struct

```rust
pub enum RedisValue {
    BulkString(Vec<u8>),
    Array(Vec<RedisValue>),
    Integer(i64),
    SimpleString(String),
    Error(String),
    Null,
}
```

## Tasks

1. Define `RedisValue` enum with all 6 variants
2. Implement `Clone`, `Debug`, `Eq`, `PartialEq`, `Hash` for `RedisValue`
3. Implement `Default` (return `Null`)
4. Add `is_null()`, `is_error()`, `is_integer()` accessor methods
5. Add `as_integer()`, `as_str()`, `as_bytes()`, `as_array()` accessor methods returning `Option<T>`

## Verification

- `cargo test -p base` — at least 5 unit tests:
  - `test_redis_value_integer_variant` — create Integer, verify variant
  - `test_redis_value_bulk_string_variant` — create BulkString, verify bytes
  - `test_redis_value_array_variant` — create nested array, verify structure
  - `test_redis_value_is_null` — Null returns true from is_null()
  - `test_redis_value_clone` — clone and verify equality
- `cargo clippy -p base` — zero warnings
