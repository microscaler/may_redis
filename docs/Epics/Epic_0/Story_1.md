# Story 0.1 — Workspace Cargo.toml

**Objective:** Create the root `Cargo.toml` workspace definition with all 6 crates listed as members.

**Epic:** 0 — Scaffolding

**Dependencies:** None

**Source docs:** `docs/08-module-structure.md`, `docs/11-dependencies.md`

## Code Anchors

- `Cargo.toml` — workspace definition
- `crates/base/Cargo.toml`
- `crates/codec/Cargo.toml`
- `crates/protocol/Cargo.toml`
- `crates/connection/Cargo.toml`
- `crates/client/Cargo.toml`
- `crates/may-redis/Cargo.toml`

## Tasks

1. Create root `Cargo.toml` with workspace members array containing all 6 crates
2. Define `[workspace.package]` with shared version (0.1.0), edition (2021), license (MIT OR Apache-2.0), repository URL
3. Define `[workspace.dependencies]` with shared dependency versions: bytes = "1.7", log = "0.4", may = { version = "0.3", default-features = false }, socket2 = "0.5"
4. Define internal crate path aliases: base, codec, protocol, connection, client
5. Create each crate's `Cargo.toml` with correct `[dependencies]` referencing workspace deps and internal crates
6. Create `crates/may-redis/Cargo.toml` with feature flags (connection, client, pool, test)

## Verification

- `cargo build --workspace` succeeds
- `cargo check -p base` succeeds
- `cargo check -p codec` succeeds
- `cargo check -p protocol` succeeds
- `cargo check -p connection` succeeds
- `cargo check -p client` succeeds
- `cargo check -p may-redis` succeeds
- All 6 crates appear in `cargo metadata --format-version 1 | jq '.packages[].name'`
