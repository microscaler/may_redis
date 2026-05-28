# Story 11.9 — Document pub/sub commands require dedicated connection

**Objective:** Add documentation to the pub/sub commands in the `Commands` trait warning that they produce correct wire format but require a dedicated connection (pub/sub state machine not implemented in connection loop).

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding P3, INFO)

**Finding:** P3 — SUBSCRIBE/UNSUBSCRIBE/PSUBSCRIBE/PUNSUBSCRIBE build command bytes but the connection loop doesn't implement the pub/sub state machine (switching from request-response to push mode). These commands will produce correct wire format but won't work correctly with the current connection loop.

## Functional Requirements

- [ ] Add `# Warning` section to `pub fn subscribe` documenting that it produces correct RESP but requires a dedicated connection
- [ ] Add `# Warning` section to `pub fn unsubscribe` with the same caveat
- [ ] Add `# Warning` section to `pub fn psubscribe` with the same caveat
- [ ] Add `# Warning` section to `pub fn punsubscribe` with the same caveat
- [ ] The warning must explain that Redis enters pub/sub mode which changes the protocol (push notifications instead of request-response)
- [ ] Suggest that users need a separate connection instance for pub/sub operations

## Non-Functional Requirements

- [ ] Warnings must be clear, concise, and reference the protocol-level reason (RESP push notifications)
- [ ] Don't add implementation — this is documentation only

## Code Anchors

- `src/protocol/commands.rs` — The pub/sub command definitions

## Tasks

1. Add `# Warning` / `# Panics` documentation to `subscribe`, `unsubscribe`, `psubscribe`, `punsubscribe`
2. Document the pub/sub protocol mode switch
3. Reference `may_postgres` or `redis` crate docs if they handle this differently

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No `clippy::missing_safety_doc` (these are unsafe operations from a protocol perspective)

### Format

- [ ] `cargo fmt --all --check` — clean

### Tests

- [ ] No behavioral changes — documentation only
- [ ] `cargo test --lib --all-features` — all tests pass

### Expected Results

- 4 commands documented with pub/sub warnings
- Callers understand the connection limitation
- Zero behavioral changes
