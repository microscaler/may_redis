# Story 1.1 — RedisValue enum

**Objective:** Implement the `RedisValue` enum representing all Redis data types. This is the single most important type in the crate.

**Epic:** 1 — Base Crate

**Dependencies:** Epic 0 (scaffolding)

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/01-protocol-analysis.md`, `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `src/core/value.rs` — `pub enum RedisValue { ... }`
- `src/core/value.rs` — `impl RedisValue` blocks

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

- [x] Define `RedisValue` enum with all 6 variants
- [x] Implement `Clone`, `Debug`, `Eq`, `PartialEq`, `Hash` for `RedisValue`
- [x] Implement `Default` (return `Null`)
- [x] Add `is_null()`, `is_error()`, `is_integer()` accessor methods
- [x] Add `as_integer()`, `as_str()`, `as_bytes()`, `as_array()` accessor methods returning `Option<T>`

## Verification

- All 147 tests pass including `RedisValue` tests:
  - `test_redis_value_integer_variant`
  - `test_redis_value_bulk_string_variant`
  - `test_redis_value_array_variant`
  - `test_redis_value_is_null`
  - `test_redis_value_clone`
- `cargo clippy` — zero warnings
