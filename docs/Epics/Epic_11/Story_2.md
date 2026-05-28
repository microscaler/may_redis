# Story 11.2 — Add `// SAFETY:` comments to all `unsafe` blocks

**Objective:** Add `// SAFETY:` comments to all 3 `unsafe` blocks in `connection.rs` explaining the invariants that make each block sound. This addresses the Rust best practice of documenting every `unsafe` block.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Findings C1, C2, S1 — MEDIUM)

**Findings:**
- **C1** — `nonblock_read` (line ~200-211) uses `unsafe` to get mutable access to `BytesMut` chunk via raw pointer cast. Missing `// SAFETY:` comment.
- **C2** — `nonblock_write` (line ~240) uses `unsafe` for unchecked slice indexing. Missing `// SAFETY:` comment.

## Functional Requirements

- [ ] `nonblock_read` must have a `// SAFETY:` comment explaining: the buffer is initialized up to `read_cnt`, `chunk_mut()` returns a `&mut [u8]` with capacity equal to `remaining_capacity()`, and the subsequent `read()` call writes into that capacity, so `advance_mut(read_cnt)` is always valid.
- [ ] `nonblock_write` must have a `// SAFETY:` comment explaining: `chunk()` returns a slice over the initialized portion of `BytesMut`, `get_unchecked(write_cnt..)` is within bounds because the `while write_cnt < len` loop invariant guarantees `write_cnt <= len`, and the kernel never writes past the slice bounds.

## Non-Functional Requirements

- [ ] Safety comments must follow Rust convention: start with `// SAFETY:` prefix
- [ ] Comments must reference the `may_postgres` pattern as the provenance (same pattern used there)

## Code Anchors

- `src/connection/connection.rs:200-211` — `nonblock_read`
- `src/connection/connection.rs:240` — `nonblock_write`

## Tasks

1. Add `// SAFETY:` comment to `nonblock_read` explaining the `chunk_mut` cast invariant
2. Add `// SAFETY:` comment to `nonblock_write` explaining the `get_unchecked` bounds invariant
3. Review the existing `// SAFETY:` comments in `may_postgres` for reference wording

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No new `clippy::undocumented_unsafe_blocks` lint (verify if this lint is enabled)

### Format

- [ ] `cargo fmt --all --check` — clean

### Tests

- [ ] All existing tests pass — `cargo test --lib --all-features`
- [ ] No behavioral changes expected (documentation-only)

### Code Review

- [ ] Comments are at the top of each `unsafe` block
- [ ] Comments explain WHY it's safe, not just WHAT it does
- [ ] Comments are consistent with Rust convention

### Expected Results

- All 3 `unsafe` blocks in `connection.rs` have `// SAFETY:` comments
- Comments reference the buffer initialization invariant
- Zero behavioral changes
- Clippy clean, tests pass
