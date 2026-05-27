# Story 6.1 — Full workspace test pass

**Objective:** Ensure all crates pass `cargo test --workspace` with all features enabled.

**Epic:** 6 — Integration & Migration

**Dependencies:** Epic 5 (client) — all prior epics complete

**Source docs:** `docs/10-test-strategy.md`

## Code Anchors

- `Cargo.toml` — test configuration
- All crate test files

## Tasks

1. Run `cargo test --workspace` — verify every crate compiles and tests pass
2. Run `cargo test --workspace --features test` — verify InMemoryClient tests pass
3. Fix any clippy deny-level warnings across all crates
4. Verify `cargo doc --workspace --no-deps` builds without errors
5. Verify `cargo fmt --check` passes on all files
6. Add `#[cfg(test)]` module to each crate with doctests where applicable

## Verification

- `cargo test --workspace` — 100% pass rate
- `cargo test --workspace --features test` — 100% pass rate
- `cargo clippy --workspace --all-targets --all-features` — zero deny-level warnings
- `cargo fmt --check` — all files formatted
- `cargo doc --workspace --no-deps` — builds without warnings
