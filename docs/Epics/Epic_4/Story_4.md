# Story 4.4 — Integration: end-to-end connection test

**Objective:** Full integration test connecting to a real Redis server and verifying the connection loop works.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.3 — epoll connection loop body must be implemented and passing tests.

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_4/Story_0.md`

## Requirements

### Functional Requirements

- **FR-1:** Integration tests must connect to a real Redis server on `localhost:6379`
- **FR-2:** Tests must verify the connection is established without error
- **FR-3:** Tests must verify a request can be sent and a response received
- **FR-4:** Tests must verify the connection can be dropped cleanly

### Non-Functional Requirements

- **NFR-1:** Integration tests must be gated behind a `integration-tests` feature flag
- **NFR-2:** Tests must compile and link even without Redis running (only fail at runtime)
- **NFR-3:** Each test must call `FLUSHDB` before execution for isolation

## Code Anchors

- `crates/connection/tests/connection_tests.rs` — integration test module

## Integration Test Plan

### test_connection_established

- Connect to `127.0.0.1:6379`
- Verify `Connection::connect()` returns `Ok(Connection)`
- Verify `conn.id()` returns a valid file descriptor

### test_ping_command

- Connect to Redis
- Send a `PING` command via the connection
- Verify the response is `+PONG\r\n` (SimpleString "PONG")

### test_connection_close

- Connect to Redis
- Send a command
- Drop the connection
- Verify the connection loop coroutine terminates cleanly (no deadlock, no panic)

## Acceptance Criteria

### Functional Acceptance Criteria

- [ ] **FR-1:** Integration tests connect to `127.0.0.1:6379` using `TcpConnector::connect()`
- [ ] **FR-2:** `test_connection_established` passes when Redis is running on localhost:6379
- [ ] **FR-3:** `test_ping_command` sends a `PING` command and verifies the response is `SimpleString("PONG")`
- [ ] **FR-4:** `test_connection_close` drops the connection and verifies the coroutine terminates without panic

### Code Quality Acceptance Criteria

- [ ] **CQ-1:** Integration tests are gated behind `#[cfg(feature = "integration-tests")]`
- [ ] **CQ-2:** `cargo test -p connection --no-run` compiles without errors even without Redis running
- [ ] **CQ-3:** `cargo clippy -p connection --all-targets --all-features` — zero warnings
- [ ] **CQ-4:** At least 3 integration tests covering connect, send, and close

## Verification

- `cargo test -p connection` — unit tests pass
- `cargo test -p connection --features integration-tests` — integration tests pass when Redis is running
- `cargo clippy -p connection` — zero warnings
- `cargo build --workspace` — full workspace builds without errors
