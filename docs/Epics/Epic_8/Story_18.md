# Story 8.18 — Pipeline Partial Error Handling

**Objective:** Add support for partial error handling in pipelines. Currently, `Pipeline::execute_raw()` blocks on ALL `rx.recv()` calls before returning, so if any single command hangs, all responses are blocked. Add an option to collect responses as they arrive and return partial results with errors for any commands that failed or timed out.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.9 (command execution timeout infrastructure).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #18, MEDIUM — API design), `src/client/pipeline.rs`

## The Problem

```rust
pub fn execute_raw(&mut self) -> Result<Vec<RedisValue>, RedisError> {
    // ... send all commands ...
    for rx in std::mem::take(&mut self.receivers) {
        let response = rx.recv()...;  // blocks on EVERY response
        responses.push(response);
    }
    Ok(responses)
}
```

If command #10 hangs forever (due to Story 8.9's timeout not being applied), `execute_raw()` blocks on `rx.recv()` for command #10. Commands #1-9 responses are received but never returned to the caller.

**Impact:** A single stuck command kills the entire pipeline batch. No partial results, no error propagation for early commands.

## Functional Requirements

1. **Option A (preferred):** Add `Pipeline::execute_raw_with_timeout(timeout: Duration)` that:
   - Sends all commands
   - Spawns a timeout coroutine
   - Collects responses as they arrive (in order)
   - If timeout fires, returns `Err(ExecutionTimeout)` with whatever responses were collected
   - If any `rx.recv()` fails (channel closed, timeout), records the error in a `Vec<Result<RedisValue, RedisError>>`

2. **Simpler Option B:** Add `Pipeline::execute_raw_partial()` that returns `Vec<Result<RedisValue, RedisError>>`:
   - Each element is `Ok(response)` for successful commands
   - Each element is `Err(error)` for failed commands
   - Never blocks on a single command — all commands execute concurrently via epoll
   - Returns when all responses arrive OR any error occurs

3. **Recommended:** Option B — it's simpler, doesn't require timeout infrastructure (which is Story 8.9), and solves the partial results problem.

## Non-Functional Requirements

1. **Zero may dependency in protocol** — timeout logic lives in `client/`.
2. **No breaking change to existing API** — `execute()` and `execute_raw()` remain unchanged.
3. **Order preserved** — responses are returned in command order.
4. **Error granularity** — each command's result is independently tracked.

## Code Anchors

- `src/client/pipeline.rs` — `Pipeline::execute_raw()`, `Pipeline::execute()`

## Tasks

1. Add `Pipeline::execute_raw_results(&mut self) -> Vec<Result<RedisValue, RedisError>>`.
2. For each command, send it and start a separate `may::go!` coroutine to wait for the response.
3. Collect results in order using shared state (Arc<Mutex<Vec<Option<...>>>>).
4. Each command waits independently — if one times out or errors, others continue.
5. Return `Vec<Result<RedisValue, RedisError>>` where each entry corresponds to a command.
6. Add `FromPipelineResponse` impl for `Vec<Result<T, RedisError>>`.

## Unit Test Plan

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `partial_all_ok` | 3 commands, all succeed | `vec![Ok(v1), Ok(v2), Ok(v3)]` |
| `partial_one_error` | 3 commands, #2 errors | `vec![Ok(v1), Err(e), Ok(v3)]` |
| `partial_count` | 5 commands | Returns 5 results (one per command) |
| `partial_order` | Commands SET, GET, INCR | Results in SET, GET, INCR order |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `execute_raw_results()` returns one Result per command
- [ ] Order is preserved
- [ ] Individual command errors don't affect other commands
- [ ] Existing `execute_raw()` and `execute()` unchanged
