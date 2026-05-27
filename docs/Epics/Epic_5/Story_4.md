# Story 5.4 — Integration tests and end-to-end verification

**Objective:** Full end-to-end integration tests connecting to a real Redis server and verifying the complete client pipeline works correctly.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.1 (RedisClient), Story 5.2 (Pipeline API), Story 5.3 (InMemoryClient)

**Source docs:** `docs/10-test-strategy.md`

## Requirements

### Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| FR-1 | Integration tests require a running Redis server on `localhost:6379` | P0 |
| FR-2 | Each integration test calls `FLUSHDB` before and after for isolation | P0 |
| FR-3 | Test `SET` + `GET` roundtrip end-to-end | P0 |
| FR-4 | Test `INCR` end-to-end | P0 |
| FR-5 | Test `DEL` end-to-end | P0 |
| FR-6 | Test `EXISTS` end-to-end | P0 |
| FR-7 | Test `TTL` + `EXPIRE` end-to-end | P0 |
| FR-8 | Test `KEYS` pattern matching end-to-end | P1 |
| FR-9 | Test `DBSIZE` end-to-end | P1 |
| FR-10 | Test `FLUSHDB` end-to-end | P1 |
| FR-11 | Test `PING` end-to-end | P1 |
| FR-12 | Test pipeline with multiple commands against real Redis | P0 |
| FR-13 | Test concurrent coroutines sharing one client | P1 |

### Non-Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| NFR-1 | Integration tests marked with `#[ignore]` by default (run with `cargo test -- --ignored`) | P0 |
| NFR-2 | No `unwrap()` in integration test assertions | P1 |
| NFR-3 | Tests are deterministic and order-independent | P1 |
| NFR-4 | Tests must pass on any machine with Redis 6+ running on localhost:6379 | P0 |

## Implementation Tasks

- [ ] Add `#[cfg(test)]` integration test module in `crates/client/src/client.rs`
- [ ] Implement `test_integration_ping` — sends PING, expects PONG
- [ ] Implement `test_integration_set_get` — SET + GET roundtrip
- [ ] Implement `test_integration_incr` — INCR auto-create and increment
- [ ] Implement `test_integration_exists_del` — EXISTS + DEL for present and missing keys
- [ ] Implement `test_integration_dbsize` — DBSIZE before/after SET
- [ ] Implement `test_integration_set_ex_ttl` — SET EX + TTL verification
- [ ] Implement `test_integration_keys` — KEYS pattern matching
- [ ] Implement `test_integration_send_sync_clone` — Clone + shared use across client copies
- [ ] Implement `test_integration_error_propagation` — INCR on string returns error
- [x] Gate all integration tests with `#[ignore]` attribute
- [ ] Implement `test_integration_pipeline` — multiple commands in pipeline
- [ ] Implement `test_integration_concurrent` — multiple coroutines sharing one client

## Verification

### Integration Tests (9 implemented, 8 passing, 1 hung)

All tests located in `crates/client/src/client.rs` under `mod tests`.
Run with: `cargo test -p client -- --ignored --test-threads=1`

| Test | Command | Status | Notes |
|---|---|---|---|
| `test_integration_ping` | PING | **PASS** | Returns "PONG" |
| `test_integration_set_get` | SET + GET | **PASS** | Roundtrip verified |
| `test_integration_incr` | INCR | **PASS** | Auto-create + increment (1, 2) |
| `test_integration_exists_del` | EXISTS + DEL | **PASS** | Present → true, missing → false, after DEL → false |
| `test_integration_dbsize` | DBSIZE | **PASS** | 0 → 2 after two SETs |
| `test_integration_set_ex_ttl` | SET EX + TTL | **PASS** | TTL within expected range |
| `test_integration_send_sync_clone` | Clone + shared use | **PASS** | Cloned client can SET/GET |
| `test_integration_error_propagation` | INCR on string | **PASS** | Returns error, not crash |
| `test_integration_keys` | KEYS | **HANGS** | See Blocked section below |
| `test_integration_pipeline` | Pipeline batch | **NOT STARTED** | Blocked on Story 5.2 |
| `test_integration_concurrent` | Multiple coroutines | **NOT STARTED** | Requires may coroutine context |

### Test Harness

Tests run inside `run_may()` which spawns the test body as a `may::coroutine::spawn` coroutine and uses `may::sync::SyncFlag` to signal completion. This is required because `may::sync::spsc::Receiver::recv()` falls through to `std::thread::park()` when called outside a may coroutine context, blocking the std thread and preventing the connection loop from running. `SyncFlag::wait()` yields cooperatively within the may scheduler.

### Commands

```bash
# With Redis running on localhost:6379:
cargo test -p client -- --ignored --test-threads=1

# Without Redis (only unit tests):
cargo test -p client
```

### Lint & Build

- [x] `cargo test -p client` — all unit tests pass (2 passed)
- [x] `cargo clippy -p client` — zero warnings
- [x] `cargo fmt -p client` — formatted
- [ ] `cargo test -p client -- --ignored` — 8/9 pass, 1 hangs on KEYS
- [ ] `cargo clippy --workspace --all-targets --all-features` — zero warnings

## Blocked / Pending

### test_integration_keys hangs indefinitely

**Symptom:** The test hangs on `client.execute(client.keys("user:*"))`. The `rx.recv()` blocks waiting for a response that never arrives.

**What is confirmed working:**
- The same KEYS command works via raw TCP — Redis returns `*2\r\n$6\r\nuser:2\r\n$6\r\nuser:1\r\n`
- `RESPReader::read_value()` correctly parses multi-bulk arrays (`read_array` test exists and passes)
- `FromRedisValue` has an impl for `Vec<String>`
- All other tests (PING, SET, GET, INCR, EXISTS, DEL, DBSIZE, SET EX, TTL, Clone, error propagation) pass

**What is NOT confirmed (the hang point):**
- The may coroutine scheduler may not be running the connection loop when `SyncFlag::wait()` is used. The `spawn` call creates a coroutine, but `SyncFlag::wait()` on the test thread might fall back to `std::thread::park()` instead of yielding to the may scheduler.
- The `rx.recv()` in `execute()` might not be properly cooperative inside the spawned coroutine context.

**Fix needed:** Investigate whether `may::coroutine::scope` is required instead of `spawn` + `SyncFlag` to properly share the may scheduler between the test coroutine and the connection loop. Or add a timeout to `rx.recv()` and use `recv_timeout` to diagnose whether the channel is just empty vs. the scheduler never running.

### test_integration_pipeline and test_integration_concurrent

These are blocked on Story 5.2 (Pipeline API) — `Pipeline` struct does not exist yet.
