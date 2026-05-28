# Story 11.8 — Name magic buffer constants in `process_req`

**Objective:** Replace the magic numbers 512 and 65536 in `process_req` with named constants for maintainability.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding C4, LOW)

**Finding:** C4 — `process_req` reserves 65536 bytes when capacity drops below 512. The magic numbers should be named constants for maintainability.

## Functional Requirements

- [ ] Define `const BUFFER_REFILL_THRESHOLD: usize = 512;` — the minimum buffer capacity before refilling
- [ ] Define `const BUFFER_REFILL_AMOUNT: usize = 65536;` — the number of bytes to reserve during refill
- [ ] Replace the inline magic numbers with these constants in `process_req`
- [ ] The buffer management logic must remain functionally identical

## Non-Functional Requirements

- [ ] Constants should be placed at the top of the file or in the same module as `process_req`
- [ ] Constants must have `//` comments explaining their purpose

## Code Anchors

- `src/connection/connection.rs:163-166` — The `process_req` function with magic numbers:
  ```rust
  if rem < 512 {
      write_buf.reserve(65536 - rem);
  }
  ```

## Tasks

1. Add `const BUFFER_REFILL_THRESHOLD: usize = 512;` with a comment
2. Add `const BUFFER_REFILL_AMOUNT: usize = 65536;` with a comment
3. Replace `rem < 512` with `rem < BUFFER_REFILL_THRESHOLD`
4. Replace `65536 - rem` with `BUFFER_REFILL_AMOUNT - rem`
5. Consider if these constants belong in `mod.rs` (shared) or `connection.rs` (used here only)

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No `clippy::match_same_arms` or similar numeric lint fires

### Format

- [ ] `cargo fmt --all --check` — clean

### Tests

- [ ] All existing tests pass — `cargo test --lib --all-features`
- [ ] No behavioral changes (constants have same values)

### Expected Results

- Magic numbers replaced with named constants
- Constants are documented with purpose comments
- Zero behavioral changes
- Clippy clean, all tests pass
