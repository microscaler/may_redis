# Story 9.5 — JSF Lint Profile for may-redis

**Objective:** Add a JSF-inspired lint configuration to enforce the safety rules identified in the BRRTRouter JSF analysis. This creates an automated gate that prevents future violations of bounded complexity, no-unwrap-in-dispatch, and bounded complexity rules.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 9 Story 1 (no-panic pipeline), Story 2 (bounded builder).

**Source docs:**
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — "BRRTRouter-SAFE" lint profile
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — Clippy configuration for JSF thresholds
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — Allocation audit findings

## The Problem

There is currently no automated enforcement of JSF safety rules. The codebase happens to be compliant for now, but without lint gates, future changes can silently introduce panics, long functions, or hot-path allocations.

## Functional Requirements

1. Add `clippy.toml` with JSF-inspired thresholds:
   - `cognitive-complexity-threshold = 20` (AV Rule 3)
   - `too-many-arguments-threshold = 8`
   - `stack-size-threshold = 512000` (256KB stack, JSF conservative)

2. Add `#![deny(...)]` or `#![warn(...)]` directives to the lib root for:
   - `clippy::unwrap_used` — flag unwrap calls (with allowlist for tests)
   - `clippy::expect_used` — flag expect calls
   - `clippy::panic` — flag explicit panic! calls

3. Update `Cargo.toml` lint configuration to match.

## Non-Functional Requirements

1. **No new dependencies.**
2. **Zero may dependency.**
3. **Backwards compatible** — existing code must pass.

## Code Anchors

- `src/lib.rs` — crate-level lint attributes
- `Cargo.toml` — lint configuration (already has deny-lints section)
- `clippy.toml` — new file with JSF thresholds

## Implementation

### `src/lib.rs` — Add crate-level lints

```rust
#![deny(clippy::unwrap_used)]
#![allow(clippy::unwrap_used, clippy::expect_used)] // in #[cfg(test)] modules only
```

Actually, `#![deny(clippy::unwrap_used)]` at crate level would fail on all unwrap calls including tests. Better approach: deny it globally, then allow it selectively in test modules with `#[allow(clippy::unwrap_used)]`.

### `clippy.toml` — JSF thresholds

```toml
cognitive-complexity-threshold = 20
too-many-arguments-threshold = 8
stack-size-threshold = 512000
```

### `Cargo.toml` — Verify deny-lints configuration

Current configuration already has pedantic and nursery deny. Add:
- `clippy::unwrap_used` = deny
- `clippy::expect_used` = deny
- `clippy::panic` = deny (for production code only)

## Verification Checklist

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `cargo test --lib` — all tests pass
- [ ] `clippy.toml` exists with JSF thresholds
- [ ] `src/lib.rs` has JSF lint directives
- [ ] Test modules use `#[allow(clippy::unwrap_used)]` for test fixtures
- [ ] Production code has zero unwrap/expect/panic
