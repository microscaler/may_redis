# Story 1.4 — Full FromRedisValue type coverage

**Objective:** Complete the `FromRedisValue` implementation for all types used by the Sesame-IDAM command set.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.3

**Source docs:** `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `crates/base/src/from_redis_value.rs` — additional impls

## Tasks

1. Implement `FromRedisValue` for `Vec<String>` — extract from Array variant, each element from RedisValue → String
2. Implement `FromRedisValue` for `Vec<i64>` — extract from Array variant, each element from RedisValue → i64
3. Implement `FromRedisValue` for `Option<String>` — Null → None, BulkString → Some(string)
4. Implement `FromRedisValue` for `usize` — extract from Integer, handle negative (return error or 0)
5. Implement `FromRedisValue` for `Vec<RedisValue>` — direct from Array variant

## Verification

- `cargo test -p base` — at least 5 additional unit tests:
  - `test_from_redis_value_array_to_vec_string` — Array of 2 BulkStrings → Vec<String>
  - `test_from_redis_value_array_to_vec_i64` — Array of 2 Integers → Vec<i64>
  - `test_from_redis_value_null_to_option_string_none` — Null → None
  - `test_from_redis_value_bulk_string_to_option_string_some` — BulkString → Some("hello")
  - `test_from_redis_value_array_to_vec_redis_value` — Array → Vec<RedisValue>
- `cargo test -p base` total should be 18+ tests
- `cargo clippy -p base` — zero warnings
- `cargo doc -p base` — all public items documented
