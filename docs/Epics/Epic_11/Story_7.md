# Story 11.7 — Add `// SAFETY:` comment for `Connection::drop` cancellation

**Objective:** Add `// SAFETY:` comment to the `Connection::drop` implementation explaining the safety contract of may's `rx.cancel()` API, and document the behavior regarding in-flight requests.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Findings C3, S2 — LOW)

**Findings:**
- **C3** — `Connection::drop` uses `unsafe { rx.cancel() }`. Could leave partial state if the loop is mid-write. The safety contract (may guarantees cancellation at yield points) should be documented.
- **S2** — `Connection::drop` cancels the coroutine but doesn't ensure in-flight requests receive error responses. Any request already dequeued but not yet sent over TCP will be silently dropped.

## Functional Requirements

- [ ] Add `// SAFETY:` comment to the `unsafe { rx.cancel() }` block in `Connection::drop`
- [ ] Document the cancellation safety contract: may guarantees cancellation at coroutine yield points only
- [ ] Document the in-flight request behavior: requests already dequeued from the queue but not yet written to TCP are silently dropped on cancellation
- [ ] No behavioral changes — this is documentation only

## Non-Functional Requirements

- [ ] The comment must explain WHY the unsafe is safe (may's coroutine model guarantees safe cancellation)
- [ ] The comment must warn about the in-flight request caveat for callers who need graceful shutdown

## Code Anchors

- `src/connection/connection.rs:141-145` — `impl Drop for Connection` with `unsafe { rx.cancel() }`

## Tasks

1. Add `// SAFETY:` comment to the `unsafe { rx.cancel() }` call
2. Comment should reference: (a) may's cancellation semantics, (b) in-flight request drop behavior
3. Optionally: consider adding a `shutdown()` method that signals the loop to drain pending requests before cancellation (nice-to-feature, can be deferred)

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No new `clippy::undocumented_unsafe_blocks` lint (if enabled)

### Format

- [ ] `cargo fmt --all --check` — clean

### Tests

- [ ] All existing tests pass — `cargo test --lib --all-features`
- [ ] No behavioral changes expected (documentation-only)

### Expected Results

- `Connection::drop` has a `// SAFETY:` comment explaining the may cancellation contract
- Callers are warned about in-flight request behavior
- Zero behavioral changes
