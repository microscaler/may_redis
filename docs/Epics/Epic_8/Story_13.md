# Story 8.13 — Add Pipeline Self-Invalidate Prevention

**Objective:** Improve `Pipeline` API to prevent accidental reuse after `execute()`. Currently, after calling `execute()` or `execute_raw()`, the internal state is cleared via `std::mem::take()`. If a user accidentally calls `add()` again after `execute()`, the new command is silently added but not part of the executed batch. Make the API safer by either consuming `self` in `execute()` or adding a clear guard.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None (pure test + API improvement in codec layer).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #9, MEDIUM), `src/client/pipeline.rs` lines 80-113

## The Problem

```rust
pub fn execute_raw(&mut self) -> Result<Vec<RedisValue>, RedisError> {
    for (data, tx) in std::mem::take(&mut self.commands)...  // clears
    ...
}

pub fn add(&mut self, cmd: CommandBuilder) {
    self.commands.push(data);  // works on cleared state
}
```

After `execute_raw()`, `self.commands` is empty. If a user calls `add()` again:
1. The new command is added to `self.commands`
2. But `self.senders` and `self.receivers` were also cleared by `mem::take()`
3. A subsequent `execute()` call would try to zip empty `senders` with the new `commands` → only the new command is sent, the others are silently lost

**Impact:** Hard-to-debug bug where pipeline commands are silently dropped after a double-execute.

## Functional Requirements

1. **Option A (preferred):** Change `execute()` to consume `self` — takes `self` by value, not `&mut self`. After `execute()`, the pipeline is moved and cannot be reused.
   - `pub fn execute<T: FromPipelineResponse>(self) -> Result<T, RedisError>`
   - `pub fn execute_raw(self) -> Result<Vec<RedisValue>, RedisError>`
2. **Option B (alternative):** Add a `bool consumed: bool` field. `add()` panics if already consumed with a clear message: "Pipeline has already been executed".
3. **Both approaches** must maintain backwards compatibility for `&mut Pipeline` callers.

Recommended: Option A with a builder-pattern alternative where users can create a new `Pipeline` per batch.

## Non-Functional Requirements

1. **Breaking change consideration** — changing `execute(&mut self)` to `execute(self)` is a minor breaking change. Since this is pre-1.0, breaking changes are acceptable.
2. **Alternative API** — if consuming `self` is too breaking, provide a `Pipeline::new(connection)` method that users can call per batch.
3. **No may dependency** — `pipeline.rs` already imports `may`.

## Code Anchors

- `src/client/pipeline.rs` — `Pipeline::execute()`, `Pipeline::execute_raw()`, `Pipeline::add()`

## Tasks

1. **Choose approach:** Go with `execute(self)` (consume) — it's the cleanest API.
2. Change `execute()` and `execute_raw()` to take `self` instead of `&mut self`.
3. Update all callers (tests, integration tests) to use `pipeline.execute()` instead of `pipeline.execute()`.
4. Add a test that verifies re-adding after execute is impossible (API-level enforcement).

## Unit Test Plan

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `pipeline_execute_consumes` | Build pipeline, call `execute()` | Compiles, pipeline is consumed |
| `pipeline_cannot_add_after_execute` | Build pipeline, call `execute()`, then `add()` | Does not compile (value moved) |
| `pipeline_new_per_batch` | Create new Pipeline for each batch | Works as expected |
| `pipeline_tuple_decoding` | 3 commands, `execute::<((), (), i64)>()` | Correct tuple decode |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Pipeline cannot be reused after `execute()` (compile-time enforcement with consume approach)
- [ ] Each batch requires a fresh `Pipeline` instance
- [ ] No panic, no silent command loss
