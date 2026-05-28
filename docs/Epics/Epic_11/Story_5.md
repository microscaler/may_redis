# Story 11.5 — Remove dead `src/connection/epoll.rs` file

**Objective:** Remove the dead file `src/connection/epoll.rs` which is not imported by any module in the crate.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding A3, INFO)

**Finding:** A3 — `src/connection/epoll.rs` is declared in directory listing but unused (no `pub mod epoll` in `connection/mod.rs`). It contains only a one-line comment: `// Epoll — Epoll-based event loop for the connection`.

## Functional Requirements

- [ ] Delete `src/connection/epoll.rs`
- [ ] Verify no other file references this module (no `mod epoll` anywhere)
- [ ] No other code depends on the contents of this file (it only contains a one-line comment)

## Non-Functional Requirements

- [ ] No behavioral changes — this file is dead code, unused everywhere
- [ ] Git history preserved — the file still exists in git log for reference

## Code Anchors

- `src/connection/epoll.rs` — The dead file (1 line, comment only)
- `src/connection/mod.rs` — Verify no `mod epoll` declaration

## Tasks

1. Verify `src/connection/mod.rs` does not declare `pub mod epoll`
2. Verify no other file in the crate references `connection::epoll` or `epoll::`
3. Delete `src/connection/epoll.rs`
4. Verify the crate still builds

## Verification

### Build

- [ ] `cargo build` — compiles successfully
- [ ] `cargo test --lib --all-features` — all tests pass

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- `src/connection/epoll.rs` removed
- Zero behavioral changes
- All 357+ tests still pass
