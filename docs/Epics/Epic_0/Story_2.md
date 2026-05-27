# Story 0.2 — Module structure and lib.rs stubs

**Objective:** Create the `src/` directory structure under each crate with `lib.rs` files containing module declarations and `//!` documentation.

**Epic:** 0 — Scaffolding

**Dependencies:** None

**Source docs:** `docs/08-module-structure.md`

## Code Anchors

- `crates/base/src/lib.rs` — module declarations, `//!` docs, mermaid diagram
- `crates/codec/src/lib.rs` — module declarations, `//!` docs, mermaid diagram
- `crates/protocol/src/lib.rs` — module declarations, `//!` docs, mermaid diagram
- `crates/connection/src/lib.rs` — module declarations, `//!` docs, mermaid diagram
- `crates/client/src/lib.rs` — module declarations, `//!` docs, mermaid diagram
- `crates/may-redis/src/lib.rs` — re-exports, `//!` docs, mermaid diagram

## Architecture

Each `lib.rs` contains:
- `//!` module-level documentation
- Mermaid architecture diagram
- Example usage code block
- Module declarations (`pub mod name;`)

## Tasks

1. Create `crates/*/src/lib.rs` for all 6 crates with:
   - `//!` doc comment describing the crate's responsibility
   - Mermaid architecture diagram showing data flow
   - Example code block
   - `pub mod` declarations for all sub-modules
2. Create `crates/*/src/*.rs` module files (empty stubs with `//!` docs)
3. Verify all modules resolve (no circular deps, no unresolved imports)

## Verification

- `cargo build --workspace` succeeds with all 6 crates
- `cargo doc --workspace --no-deps` builds without errors
- All `lib.rs` files contain `//!` module-level documentation
- All `lib.rs` files contain mermaid architecture diagrams
