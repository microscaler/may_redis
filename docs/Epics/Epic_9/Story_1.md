# Story 9.1 — No-Panic Pipeline Deserialization

**Objective:** Replace all `unwrap()` calls in the production `FromPipelineResponse` trait implementations with proper `Result` error propagation. Currently, if the server returns a different number of responses than expected, the client panics instead of returning a descriptive `RedisError::Parse`.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 8 (all 20 stories complete, `cargo test --lib` 284 tests pass, clippy clean).

**Source docs:**
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — AV Rule 208: no panics in dispatch
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — "No panics (Rule 208): Result-based error handling"
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — "No panics in the hot path: unwrap, expect, panic!, assert! banned"

## The Problem

The `FromPipelineResponse` implementations use `.unwrap()` on `Iterator::next()` after validating the response count:

```rust
// pipeline.rs:172 — 4 identical patterns
let t1 = T1::from_redis_value(&responses.into_iter().next().unwrap())?;
```

The length check (`if responses.len() != N`) is correct, but `.unwrap()` is a dead code path that will panic if the logic is ever wrong. Under JSF AV Rule 208, this is a violation — dispatch code must never panic.

## Functional Requirements

1. Replace `.unwrap()` on `iter.next()` with proper error handling returning `Result<Self, RedisError>`.
2. The error should be `RedisError::Parse` with a descriptive message.
3. All existing tests must continue to pass.
4. Add a new test: `test_pipeline_wrong_count` that verifies a count mismatch returns `Err(Parse(...))` instead of panicking.

## Non-Functional Requirements

1. **No API changes** — `from_responses` signature unchanged.
2. **No new dependencies.**
3. **No may dependency change** — pipeline.rs already has may, this doesn't change that.
4. **Behavior improvement** — Current behavior: panic. New behavior: `Err(RedisError::Parse("expected N responses, got M"))`. Strictly more resilient.

## Code Anchors

- `src/client/pipeline.rs` — `impl FromPipelineResponse for (T1,)` (line ~164)
- `src/client/pipeline.rs` — `impl FromPipelineResponse for (T1, T2)` (line ~177)
- `src/client/pipeline.rs` — `impl FromPipelineResponse for (T1, T2, T3)` (line ~192)
- `src/client/pipeline.rs` — `impl FromPipelineResponse for (T1, T2, T3, T4)` (line ~210)

## Implementation

Replace this pattern in all 4 tuple impls:
```rust
let t1 = T1::from_redis_value(&responses.into_iter().next().unwrap())?;
```

With index-based access on the owned Vec (zero allocation, no `unwrap`):
```rust
let r0 = responses[0];
let t1 = T1::from_redis_value(&r0)?;
```

For tuples of 4:
```rust
let r0 = responses[0];
let r1 = responses[1];
let r2 = responses[2];
let r3 = responses[3];
let t1 = T1::from_redis_value(&r0)?;
// etc.
```

This avoids `.next().unwrap()` entirely, uses index access on the already-owned Vec (no allocation), and the count check above already guarantees bounds safety.

## Unit Test Plan

| Test Name | Scenario | Expected |
|-----------|----------|----------|
| `test_pipeline_wrong_count` | Pass responses.len() != tuple size | `Err(Parse(...))` not panic |
| `test_from_pipeline_response_single` | Single Integer(42) | `Ok((42,))` |
| `test_from_pipeline_response_pair` | Integer(1) + BulkString | `Ok((true, "hello"))` |
| `test_from_pipeline_response_triple` | Three Integers | `Ok((true, 2, 3))` |
| `test_from_pipeline_response_vec` | Three BulkStrings | `Ok(Vec<String>)` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 285+ tests pass (284 existing + 1 new)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No `unwrap()` in production `FromPipelineResponse` impls (only in `#[cfg(test)]` mod)
- [ ] Count mismatch returns `Err(Parse(...))`, not `panic!`
- [ ] No performance regression (same allocation pattern as before)
