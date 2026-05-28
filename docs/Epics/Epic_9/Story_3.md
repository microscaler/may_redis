# Story 9.3 — Bounded Allocation in ToRedisArgs

**Objective:** Replace `Vec::new()` allocations in `ToRedisArgs` implementations and their callers. The `write_redis_args` trait currently takes `&mut Vec<Vec<u8>>` which means every impl allocates a new Vec.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 9 Story 2 (bounded allocation in CommandBuilder).

**Source docs:**
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — "No heap after init: No String::new, Vec::new, format!, to_string in dispatch"
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — 14 instances of `Vec::new()` in `to_args.rs`
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — "Stack-Allocated Collections"

## The Problem

`src/core/to_args.rs` has 14 instances of `Vec::new()` across:
- `to_args.rs:175,183,191,199,207,215` — Test code (ok)
- `to_args.rs:232,244,251,268,276,284,292,312,319` — Test code (ok)

All 14 instances are in `#[cfg(test)]` modules. The production code uses the trait's `write_redis_args(&self, buf: &mut Vec<Vec<u8>>)` method but never allocates in production — the caller (CommandBuilder) passes the buffer.

Wait — let me re-check. The audit script flagged these as non-test but they may all be in test modules.

## Functional Requirements

1. Verify all `Vec::new()` in `to_args.rs` are in `#[cfg(test)]` modules.
2. If not, move the allocation to the caller (CommandBuilder) which already has a reusable buffer from Story 9.2.
3. If all are in tests, no code change needed — just document the finding.

## Non-Functional Requirements

1. **No new dependencies.**
2. **Zero may dependency.**
3. **Backwards compatible** — trait signature unchanged.

## Code Anchors

- `src/core/to_args.rs` — trait definition and all implementations
- `src/protocol/builder.rs` — consumer of `write_redis_args`

## Implementation

If production `Vec::new()` found:
1. Change `write_redis_args` to accept `&mut dyn AsRef<[u8]>` or a generic buffer trait, OR
2. Keep trait signature, add a `to_redis_bytes(&self) -> Vec<u8>` helper that each impl can use, called from CommandBuilder
3. CommandBuilder passes its pre-allocated buffer to the trait method

If all in tests:
- No change needed. Document that `to_args.rs` is JSF-compliant (zero production allocations).

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Zero `Vec::new()` in non-test production code in `to_args.rs`
- [ ] All existing `ToRedisArgs` impls still work
