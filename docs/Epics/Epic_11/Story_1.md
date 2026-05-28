# Story 11.1 — Replace `std::thread::sleep` with `may::timer::sleep`

**Objective:** Replace the blocking `std::thread::sleep(timeout)` inside the timeout coroutine spawned in `execute_with_timeout` with `may::timer::sleep`. This prevents blocking a may worker thread, which can starve the scheduler under load.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding CL1, MEDIUM)

**Finding:** CL1 — `std::thread::sleep` inside `go!` blocks a may worker thread. On a system with few may workers this could starve the scheduler.

## Functional Requirements

- [ ] `execute_with_timeout` must use `may::timer::sleep(Duration)` instead of `std::thread::sleep(Duration)` for the timeout coroutine
- [ ] The timeout semantics must remain identical: the coroutine sleeps for exactly `timeout`, then sends a signal via the `timeout_tx` spsc channel
- [ ] All existing callers (`execute_timeout`, `execute`) must continue working without code changes

## Non-Functional Requirements

- [ ] No may worker thread starvation under concurrent timeout usage
- [ ] Timeout behavior must be coroutine-friendly: `may::timer::sleep` cooperatively yields to the may scheduler

## Code Anchors

- `src/client/client.rs:103-106` — The current `go!` block with `std::thread::sleep(timeout)`

## Tasks

1. Replace `std::thread::sleep(timeout)` with `may::timer::sleep(timeout)` in the timeout coroutine (line 104)
2. Import `may::timer::sleep` if not already imported
3. Update doc comment on `execute_with_timeout` to document that timeout uses coroutine-friendly sleeping

## Verification

### Unit Tests

- [ ] Existing `cargo test --lib --all-features` must still pass (the test `test_timeout_with_may_runtime` verifies timeout behavior)
- [ ] Add new test `test_timeout_uses_may_timer` that verifies the timeout coroutine yields to the may scheduler (not a real thread)
- [ ] Test that the timeout still fires at the correct duration after the change

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] No `clippy::manual_sleep` lint fire (if it exists)

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Test

- [ ] Run with live Redis: `cargo test --lib --test-threads=1 -- --include-ignored test_integration_set_get` (or any integration test)
- [ ] Verify timeout still works: `cargo test --lib --test-threads=1 -- --include-ignored test_integration_timeout`

### Expected Results

- Zero changes to external API
- Zero changes to timeout behavior (same Duration, same error message)
- No may worker thread blocked during timeout wait
- All 357+ tests pass
- Clippy clean
- Format clean
