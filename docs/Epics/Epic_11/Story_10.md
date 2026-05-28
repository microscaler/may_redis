# Story 11.10 — Document blocking command timeout considerations

**Objective:** Add documentation to BLPOP/BRPOP and similar blocking commands warning users about the 30-second default timeout and the need to use `execute_with_timeout` with an appropriate duration.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding P4, INFO)

**Finding:** P4 — BLPOP/BRPOP timeout handling: these blocking commands may exceed the client's 30-second default timeout. Users must use `execute_with_timeout` with an appropriate duration. No documentation warns about this.

## Functional Requirements

- [ ] Add `# Timeout warning` section to `pub fn blpop` documenting the 30-second default timeout
- [ ] Add `# Timeout warning` section to `pub fn brpop` with the same warning
- [ ] Add `# Timeout warning` section to `pub fn blpop_multi` / `pub fn brpop_multi` (if they exist)
- [ ] Document that `execute_with_timeout` should be used for commands with longer blocking times
- [ ] The default timeout is 30 seconds (set in `execute()`)

## Non-Functional Requirements

- [ ] Warnings must reference the specific method to use (`execute_with_timeout`)
- [ ] Warnings must explain what happens if the timeout fires (returns `RedisError::Connection` timeout error)

## Code Anchors

- `src/protocol/commands.rs` — BLPOP/BRPOP command definitions
- `src/client/client.rs:171-173` — The `execute()` method with 30-second default timeout

## Tasks

1. Add `# Timeout warning` sections to BLPOP, BRPOP, and any other blocking commands
2. Reference `execute_with_timeout` in the documentation
3. Consider adding a `#[must_use]` attribute reminder

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Tests

- [ ] No behavioral changes — documentation only
- [ ] `cargo test --lib --all-features` — all tests pass

### Expected Results

- Blocking commands document the 30-second default timeout
- Users are directed to `execute_with_timeout` for longer waits
- Zero behavioral changes
