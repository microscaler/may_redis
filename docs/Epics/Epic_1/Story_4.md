# Story 1.4 — Full FromRedisValue type coverage

**Objective:** Complete the `FromRedisValue` implementation for all types used by the Sesame-IDAM command set.

**Epic:** 1 — Base Crate

**Dependencies:** Story 1.3

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `src/core/from_value.rs` — additional impls

## Tasks

- [x] Implement `FromRedisValue` for `Vec<String>` — extract from Array variant, each element from RedisValue → String
- [x] Implement `FromRedisValue` for `Vec<i64>` — extract from Array variant, each element from RedisValue → i64
- [x] Implement `FromRedisValue` for `Option<String>` — Null → None, BulkString → Some(string)
- [x] Implement `FromRedisValue` for `usize` — extract from Integer, handle negative (return error or 0)
- [x] Implement `FromRedisValue` for `Vec<RedisValue>` — direct from Array variant

## Verification

- All tests pass:
  - `test_from_redis_value_array_to_vec_string`
  - `test_from_redis_value_array_to_vec_i64`
  - `test_from_redis_value_null_to_option_string_none`
  - `test_from_redis_value_bulk_string_to_option_string_some`
  - `test_from_redis_value_array_to_vec_redis_value`
- Total base tests: 18+
- `cargo clippy` — zero warnings
