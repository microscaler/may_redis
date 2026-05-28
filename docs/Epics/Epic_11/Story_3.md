# Story 11.3 — Fix `mget`/`mset`/`msetnx`/`sinter`/`sunion` API consistency

**Objective:** Make `mget`, `mset`, `msetnx`, `sinter`, `sunion` take `&self` like all other `Commands` trait methods. This allows them to be called via `client.mget(...)` through the trait.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding P1, MEDIUM)

**Finding:** P1 — `mget`, `mset`, `msetnx`, `sinter`, `sunion` are associated functions (no `&self`) while all other commands take `&self`. This means `client.mget(...)` won't compile via the `Commands` trait.

## Functional Requirements

- [ ] Change `mget` signature from `fn mget<K: ToRedisArgs>(keys: &[K]) -> CommandBuilder` to `fn mget<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder`
- [ ] Change `mset` signature to take `&self`
- [ ] Change `msetnx` signature to take `&self`
- [ ] Change `sinter` signature to take `&self`
- [ ] Change `sunion` signature to take `&self`
- [ ] All other `Commands` trait methods must already take `&self` (no regression)
- [ ] Default trait implementations must work (the `impl Commands for RedisClient` block may become empty after this story plus Story 11.4)

## Non-Functional Requirements

- [ ] API must be backwards compatible — existing code using `CommandBuilder::new("MGET")` directly is unaffected
- [ ] The `#[must_use]` annotations must be preserved on all 5 methods

## Code Anchors

- `src/protocol/commands.rs:186` — `fn mget<K: ToRedisArgs>(keys: &[K])`
- `src/protocol/commands.rs:196` — `fn mset<K: ToRedisArgs, V: ToRedisArgs>(pairs: &[(K, V)])`
- `src/protocol/commands.rs:206` — `fn msetnx<K: ToRedisArgs, V: ToRedisArgs>(pairs: &[(K, V)])`
- `src/protocol/commands.rs:391` — `fn sinter<K: ToRedisArgs>(keys: &[K])`
- `src/protocol/commands.rs:401` — `fn sunion<K: ToRedisArgs>(keys: &[K])`

## Tasks

1. Add `&self` parameter to `mget`, `mset`, `msetnx`, `sinter`, `sunion` in the `Commands` trait definition
2. Add `&self` parameter to default implementations in `impl Commands for RedisClient`
3. Update any callers that invoke these via `Commands::mget(...)` — change to `client.mget(...)` or `Commands::mget(&client, ...)`
4. Check if any test code calls these methods directly

## Verification

### Unit Tests

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `cargo test --lib --all-features` — all tests pass
- [ ] Verify `Commands::mget` resolves on `&RedisClient`: `_require_commands::<RedisClient>()` test must still compile

### Integration Test

- [ ] Verify `client.mget(&["key1", "key2"])` compiles and works via the `Commands` trait:
  - Execute `client.mget(&["a", "b"])` through `client.execute(...)` and verify result
- [ ] Same for `mset`, `msetnx`, `sinter`, `sunion`

### Regression

- [ ] Verify `CommandBuilder::new("MGET").arg("a").arg("b")` still works for callers who prefer the builder pattern directly
- [ ] No changes to the RESP wire format produced

### Expected Results

- All 5 methods have `&self` signature matching the rest of the trait
- `client.mget(keys)` compiles via the `Commands` trait
- Zero changes to RESP wire format
- All existing callers continue to work
- Clippy clean, all tests pass
