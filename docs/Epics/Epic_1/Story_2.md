# Story 1.2 — RedisError + FromRedisValue trait

**Objective:** Implement the `RedisError` enum and the `FromRedisValue` trait.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.1

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `src/core/error.rs` — `pub enum RedisError { ... }`
- `src/core/from_value.rs` — `pub trait FromRedisValue`

## Structs

```rust
pub enum RedisError {
    Connection(String),
    Protocol(String),
    Parse(String),
    Other(String),
}

pub trait FromRedisValue: Sized {
    fn from_redis_value(value: &RedisValue) -> Result<Self, RedisError>;
}
```

## Tasks

- [x] Define `RedisError` enum with Connection, Protocol, Parse, Other variants
- [x] Implement `std::error::Error`, `std::fmt::Display`, `std::fmt::Debug`, `From<String>` for `RedisError`
- [x] Define `pub type RedisResult<T> = Result<T, RedisError>;`
- [x] Define `FromRedisValue` trait with `from_redis_value` method
- [x] Implement `FromRedisValue` for `i64` (extract from Integer variant, error from others)
- [x] Implement `FromRedisValue` for `String` (extract from BulkString/SimpleString, error from others)
- [x] Implement `FromRedisValue` for `()` (extract from SimpleString "OK" or Integer 1)
- [x] Implement `FromRedisValue` for `bool` (extract from Integer 0 or 1)

## Verification

- All tests pass:
  - `test_from_redis_value_integer_to_i64`
  - `test_from_redis_value_integer_to_i64_wrong_type`
  - `test_from_redis_value_bulk_string_to_string`
  - `test_from_redis_value_simple_string_to_string`
  - `test_from_redis_value_to_unit`
  - `test_from_redis_value_to_bool_true`
  - `test_from_redis_value_to_bool_false`
  - `test_from_redis_value_null_to_string`
  - `test_redis_error_display`
  - `test_redis_error_from_string`
- `cargo clippy` — zero warnings
