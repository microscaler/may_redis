# Story 12.3 — Integration test for `Connection::drop` error behavior

| Field       | Value                                              |
|-------------|----------------------------------------------------|
| **Story**   | 12.3                                               |
| **Epic**    | [Story 0](/home/casibbald/Workspace/microscaler/may_redis/docs/Epics/Epic_12/Story_0.md) (Epic overview) |
| **Dependencies** | Story 0                                        |
| **Status**  | In Progress                                        |

## Objective

Add an integration test verifying that `Connection::drop` produces correct error behavior when multiple coroutines are concurrently awaiting responses.

## Background

The code audit (finding `docs/code-review-2026-05-28.md`) identified that `Connection::drop` cancels the may coroutine but there's no test verifying that in-flight requests properly receive error responses.

The connection loop's error handling (lines 427-434, 446-454, 463-470 of `src/connection/connection.rs`) drains all pending requests with `RedisValue::Error` on fatal I/O errors, but cancellation via `may::coroutine::cancel` bypasses this cleanup path. If the connection is dropped while a coroutine is blocked on `rx.recv()`, the behavior is undefined — it could hang, panic, or silently drop.

## Source

- `docs/code-review-2026-05-28.md` — Findings **C3** and **S2** (MEDIUM)

## Findings

- **C3**: `Connection::drop` uses `unsafe { rx.cancel() }` but could leave partial state if loop is mid-write
- **S2**: `Connection::drop` cancels the coroutine but doesn't ensure in-flight requests receive error responses before cancellation

## Code Anchors

- `src/connection/connection.rs:141-155` — Drop implementation for `Connection`
- `src/connection/connection.rs:154` — `unsafe { rx.cancel() }`

## Functional Requirements

- [ ] Test that dropping a connection while coroutines await responses causes those coroutines to get an error, not hang
- [ ] Test that multiple concurrent coroutines all receive errors (not just the first)
- [ ] Test that no coroutine panics or deadlocks when connection is dropped

## Non-Functional Requirements

- [ ] Test uses `may::run` / `may::go` (never `#[tokio::test]`)
- [ ] Test must create a separate connection (not shared client) so drop is safe
- [ ] Test must use a timeout on `rx.recv()` to detect hangs (not block forever)
- [ ] Test must use `Arc` to share connection handle across coroutines

## Implementation Tasks

1. **Create test `test_connection_drop_during_request`**:
   - Create a new `Connection`
   - Spawn 3 coroutines that each send a PING command
   - Immediately drop the connection
   - Verify all 3 coroutines complete (don't hang) and receive errors

2. **Create test `test_connection_drop_during_pipeline`**:
   - Create a new `Connection`
   - Spawn 2 coroutines that each execute a pipeline of 5 commands
   - Immediately drop the connection
   - Verify all coroutines complete with errors

3. **Create test `test_connection_drop_no_panic`**:
   - Create connection, send command, drop in different order
   - Verify no panic occurs

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests (require Redis)

- [ ] `test_integration_connection_drop_during_request` — 3 concurrent PINGs, drop connection, all 3 get errors
- [ ] `test_integration_connection_drop_during_pipeline` — 2 concurrent pipelines (5 cmds each), drop, all get errors

## Acceptance Criteria

- [ ] Connection drop never causes a hang (verified by test timeout)
- [ ] Connection drop never causes a panic
- [ ] All awaiting coroutines receive an error result
- [ ] At least 3 coroutines can share one connection and survive its drop
- [ ] No std::thread::sleep anywhere in the codebase (grep check)
- [ ] No .await anywhere in the codebase
- [ ] All existing tests still pass
