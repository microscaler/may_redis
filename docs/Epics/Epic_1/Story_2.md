# Story 1.2 — RedisError + FromRedisValue trait

**Objective:** Implement the `RedisError` enum and the `FromRedisValue` trait.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.1

**Source docs:** `docs/Epics/Epic_1/Story_0.md`

## Code Anchors

- `crates/base/src/redis_error.rs` — `pub enum RedisError { ... }`
- `crates/base/src/from_redis_value.rs` — `pub trait FromRedisValue`

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

1. Define `RedisError` enum with Connection, Protocol, Parse, Other variants
2. Implement `std::error::Error`, `std::fmt::Display`, `std::fmt::Debug`, `From<String>` for `RedisError`
3. Define `pub type RedisResult<T> = Result<T, RedisError>;`
4. Define `FromRedisValue` trait with `from_redis_value` method
5. Implement `FromRedisValue` for `i64` (extract from Integer variant, error from others)
6. Implement `FromRedisValue` for `String` (extract from BulkString/SimpleString, error from others)
7. Implement `FromRedisValue` for `()` (extract from SimpleString "OK" or Integer 1)
8. Implement `FromRedisValue` for `bool` (extract from Integer 0 or 1)

## Verification

- `cargo test -p base` — at least 8 unit tests:
  - `test_from_redis_value_integer_to_i64` — Integer(42) → 42
  - `test_from_redis_value_integer_to_i64_wrong_type` — BulkString → Error
  - `test_from_redis_value_bulk_string_to_string` — BulkString(b"hello") → "hello"
  - `test_from_redis_value_simple_string_to_string` — SimpleString("OK") → "OK"
  - `test_from_redis_value_to_unit` — Integer(1) → ()
  - `test_from_redis_value_to_bool_true` — Integer(1) → true
  - `test_from_redis_value_to_bool_false` — Integer(0) → false
  - `test_from_redis_value_null_to_string` — Null → Parse error
- `test_redis_error_display` — error formatting
- `test_redis_error_from_string` — From<String> impl
- `cargo clippy -p base` — zero warnings
