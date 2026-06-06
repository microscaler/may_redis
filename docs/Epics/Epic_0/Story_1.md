# Story 0.1 — Single-Crate Cargo.toml

**Objective:** Create the `may-redis` single-crate `Cargo.toml` with all features and lint configuration.

**Epic:** 0 — Scaffolding

**Dependencies:** None

**Source docs:** `docs/adr-001-single-crate-structure.md`

**Status:** COMPLETE

## Code Anchors

- `Cargo.toml` — single crate manifest

## Tasks

1. Create `Cargo.toml` with package metadata (name, version, edition, license, repository)
2. Define `[dependencies]` with shared dependency versions: bytes = "1.7", log = "0.4", may = { version = "0.3", default-features = false }, socket2 = "0.5"
3. Define feature flags: `tls`, `cluster`, `test`
4. Add `[dev-dependencies]` with testing tools
5. Define `[lints.clippy]` with deny-lints and allows

## Verification

- `cargo check` succeeds
- `cargo clippy --lib -- -D warnings` passes
- All lints configured correctly in `[lints.clippy]`
