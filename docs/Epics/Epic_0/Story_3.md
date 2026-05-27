# Story 0.3 — Lint configuration and CI tooling

**Objective:** Configure clippy deny-lints, format configuration, and ensure the codebase enforces quality standards from day one.

**Epic:** 0 — Scaffolding

**Dependencies:** None

**Source docs:** `docs/08-module-structure.md`

## Code Anchors

- `Cargo.toml` — workspace lint configuration
- `crates/base/Cargo.toml` — crate-level lint config
- `crates/codec/Cargo.toml` — crate-level lint config
- `crates/protocol/Cargo.toml` — crate-level lint config
- `crates/connection/Cargo.toml` — crate-level lint config
- `crates/client/Cargo.toml` — crate-level lint config
- `crates/may-redis/Cargo.toml` — crate-level lint config

## Lint Configuration

Root `Cargo.toml` adds:

```toml
[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
# Allow rules that are noisy for library code
cast_precision_loss = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
module_name_repetitions = "allow"
struct_excessive_bools = "allow"
too_many_lines = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
missing_safety_doc = "allow"
```

## Tasks

1. Add `[lints.clippy]` section to root `Cargo.toml` with deny-lints and allows
2. Add `[lints.clippy]` to each crate's `Cargo.toml` with inherited workspace config
3. Ensure `cargo fmt` and `cargo clippy --workspace --all-targets --all-features` pass on empty stubs

## Verification

- `cargo fmt --check` succeeds (all files formatted)
- `cargo clippy --workspace --all-targets --all-features` succeeds (no deny-level warnings)
- All allow-listed rules are documented with comments explaining why they are allowed
