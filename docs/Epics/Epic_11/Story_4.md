# Story 11.4 — Remove redundant `impl Commands for RedisClient` method bodies

**Objective:** Remove all redundant method implementations from `impl Commands for RedisClient` in `src/client/client.rs`. The trait already provides default implementations that do the same thing — the `impl` block can be empty.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview), Story 11.3 (mget/mset API consistency — must run first)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding P2, LOW)

**Finding:** P2 — `Commands` trait re-implements every method identically in the `impl Commands for RedisClient` block. This is redundant since the trait has default implementations. The `impl` block could simply be `impl Commands for RedisClient {}`.

## Functional Requirements

- [ ] Remove all method bodies from `impl Commands for RedisClient` in `src/client/client.rs`
- [ ] The impl block becomes: `impl Commands for RedisClient {}`
- [ ] All 80+ Commands trait methods must still be callable on `RedisClient` via default trait implementations
- [ ] The `#[must_use]` annotations must be preserved (they live on the trait methods, so they remain)

## Non-Functional Requirements

- [ ] No change in behavior — every command produces identical RESP wire format
- [ ] No change in compilation speed (the impl block is trivial either way, but removing dead code improves signal-to-noise)

## Code Anchors

- `src/client/client.rs:208-284` — `impl Commands for RedisClient` block with all redundant method bodies
- `src/protocol/commands.rs` — The `Commands` trait with default implementations

## Tasks

1. Delete all method bodies from `impl Commands for RedisClient` (lines ~208-284)
2. Replace with empty impl: `impl Commands for RedisClient {}`
3. Verify the `ping` inherent method vs trait method resolution still works (see existing note at line 286-291)
4. Ensure the `#[allow(clippy::needless_pass_by_value)]` annotations are moved to the trait definition if they should apply

## Verification

### Unit Tests

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `cargo test --lib --all-features` — all tests pass
- [ ] `test_commands_trait_methods_exist` — still compiles (verifies trait is implemented)

### Clippy

- [ ] No `clippy::needless_pass_by_value` warnings introduced (annotations may need to move to trait)
- [ ] No dead code warnings — the impl block must be empty but valid

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Test

- [ ] Run with live Redis: `cargo test --lib --test-threads=1 -- --include-ignored test_integration_set_get`
- [ ] Verify `client.get()`, `client.set()`, `client.ping()`, etc. all still work via the Commands trait

### Expected Results

- `impl Commands for RedisClient` shrinks from ~80 lines to 1 line
- Zero behavioral changes
- All 80+ commands still callable
- Clippy clean, all tests pass
