# Story 8.5 — CommandBuilder::arg() Discards Extra ToRedisArgs Buffers

**Objective:** Fix `CommandBuilder::arg()` to correctly handle `ToRedisArgs` implementations that produce multiple buffers (e.g., `Vec<&str>`). Currently, `arg()` calls `.into_iter().next()` and only keeps the first buffer, silently discarding all subsequent ones.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1, 8.2 (basic type conversions present). No new dependencies.

**Source docs:** `docs/redis-implementation-audit.md` (Finding #1, CRITICAL), `src/protocol/builder.rs` lines 32-39

## The Bug

`arg()` signature:
```rust
pub fn arg<V: ToRedisArgs>(self, val: V) -> Self {
    let mut buf = Vec::new();
    val.write_redis_args(&mut buf);
    if let Some(first) = buf.into_iter().next() {  // <-- only takes FIRST element
        builder.args.push(RedisValue::BulkString(first));
    }
    builder
}
```

When `V = Vec<&str>`, `write_redis_args` iterates over elements and calls each one's `write_redis_args`, producing three `Vec<u8>` entries. But `.next()` only keeps "a", silently discarding "b" and "c".

**Workaround:** Users must use `cmd("MGET").args(&["k1", "k2", "k3"])` instead of `.arg(...)`. The `args()` method works correctly.

**Impact:** `cmd("MGET").arg(vec!["k1", "k2", "k3"])` produces `*2\r\n$4\r\nMGET\r\n$2\r\nk1\r\n` instead of `*4\r\n$4\r\nMGET\r\n$2\r\nk1\r\n$2\r\nk2\r\n$2\r\nk3\r\n`. The missing keys are silently dropped.

## Functional Requirements

1. `arg()` must collect ALL buffers from `write_redis_args()` and push each as a separate `RedisValue::BulkString` into the command args.
2. `args()` must continue to work identically (it already does).
3. `arg()` with single-element types (String, &str, i64, bool) must produce exactly one additional arg — no regression.
4. `arg()` with multi-element types (Vec<T> where T: ToRedisArgs) must produce one additional arg per element.
5. `arg()` with empty ToRedisArgs (Vec::<&str>::new()) must add zero args — no regression.

## Non-Functional Requirements

1. **Backwards compatible** — callers using `arg()` with single-value types see no change in behavior or API.
2. **Zero new dependencies** — pure code fix in `builder.rs`.
3. **No performance regression** — `arg()` must avoid unnecessary allocations; use `buf.into_iter()` (consumes, no copy).
4. **Builder pattern preserved** — `arg()` returns `Self` (new builder), does not mutate `self`.

## Code Anchors

- `src/protocol/builder.rs` lines 32-39 — `CommandBuilder::arg()`

## Tasks

1. Refactor `arg()` to iterate all buffers from `write_redis_args` instead of taking only `.next()`.
2. Write unit test: `cmd("MGET").arg(vec!["k1", "k2", "k3"])` produces 4-element array (`MGET k1 k2 k3`).
3. Write unit test: `cmd("SET").arg("key").arg(vec!["a", "b"])` produces 4-element array (`SET key a b`).
4. Write unit test: `cmd("PING").arg("single")` still produces 2-element array (regression guard).
5. Write unit test: `cmd("SET").arg(Vec::<&str>::new())` produces 1-element array (empty arg guard).

## Unit Test Plan

| Test Name | Input | Expected Wire |
|-----------|-------|---------------|
| `arg_vec_str_multi` | `cmd("MGET").arg(vec!["k1","k2","k3"])` | `*4\r\n$4\r\nMGET\r\n$2\r\nk1\r\n$2\r\nk2\r\n$2\r\nk3\r\n` |
| `arg_vec_str_single` | `cmd("GET").arg(vec!["only"])` | `*2\r\n$4\r\nGET\r\n$4\r\nonly\r\n` |
| `arg_vec_str_empty` | `cmd("GET").arg(Vec::<&str>::new())` | `*1\r\n$3\r\nGET\r\n` |
| `arg_string_single` | `cmd("SET").arg("k").arg("v")` | `*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n` |
| `arg_i64_single` | `cmd("INCR").arg(42i64)` | `*2\r\n$4\r\nINCR\r\n$2\r\n42\r\n` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all existing tests still pass (no regression)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `cmd("MGET").arg(vec!["k1","k2","k3"]).build()` produces `*4` (4 elements)
- [ ] `cmd("SET").arg("k").arg(vec![]).build()` produces `*1` (no extra element from empty vec)
- [ ] Builder pattern: chaining `.arg()` calls still works (each returns new builder)
