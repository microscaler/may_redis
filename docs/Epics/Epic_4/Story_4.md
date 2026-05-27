# Story 4.4 — Integration: end-to-end connection test

**Objective:** Full integration test connecting to a real Redis server and verifying the connection loop works.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.3 — epoll connection loop body

**Status:** COMPLETE — all integration tests pass with live Redis.

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_4/Story_0.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** Integration tests connect to a real Redis server on `localhost:6379`
- [x] **FR-2:** Tests verify the connection is established without error
- [x] **FR-3:** Tests verify a request can be sent and a response received
- [x] **FR-4:** Tests verify the connection can be dropped cleanly

### Non-Functional Requirements

- [x] **NFR-1:** Tests use the `test` feature flag where applicable
- [x] **NFR-2:** Tests compile and link without Redis running (only fail at runtime)
- [x] **NFR-3:** Each test calls `FLUSHDB` before execution for isolation

## Code Anchors

- `src/client/client.rs` — integration tests in `#[cfg(test)]` module (`test_integration_*`)

## Integration Test Plan

### test_connection_established
- [x] Connect to `127.0.0.1:6379` — passes when Redis is running
- [x] Verify `Connection::connect()` returns `Ok(Connection)`

### test_ping_command
- [x] Send a `PING` command via the connection
- [x] Verify the response is `SimpleString("PONG")`

### test_connection_close
- [x] Drop the connection and verify the coroutine terminates without panic

### Additional integration tests (11 total)
- [x] `test_integration_set_get` — SET/GET roundtrip
- [x] `test_integration_set_ex_ttl` — SET EX / TTL
- [x] `test_integration_exists_del` — EXISTS / DEL
- [x] `test_integration_incr` — INCR auto-creates key
- [x] `test_integration_keys` — KEYS with pattern
- [x] `test_integration_dbsize` — DBSIZE
- [x] `test_integration_pipeline` — Pipeline ordering
- [x] `test_integration_concurrent` — Multiple coroutines sharing one client
- [x] `test_integration_send_sync_clone` — RedisClient is Clone + Send + Sync
- [x] `test_integration_error_propagation` — Errors bubble up correctly

## Verification

- All 11 integration tests pass with Redis on localhost:6379
- Tests run with `--test-threads=1` (shared state requirement)
- `cargo clippy` — zero warnings
