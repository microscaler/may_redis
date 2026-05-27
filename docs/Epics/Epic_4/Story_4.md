# Story 4.4 — Integration: end-to-end connection test

**Objective:** Full integration test connecting to a real Redis server and verifying the connection loop works.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.3

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_4/Story_0.md`

## Code Anchors

- `crates/connection/tests/` — integration test module

## Tasks

1. Create integration test module `crates/connection/tests/connection_tests.rs`
2. Test: `test_connection_established` — connect to Redis on localhost:6379, verify no error
3. Test: `test_connection_send_request` — send a Request, verify it appears in the write buffer
4. Test: `test_response_dispatch` — simulate a server response, verify the spsc receiver gets the correct value
5. Test: `test_connection_close` — drop the connection, verify the loop terminates cleanly
6. Add `#[cfg(feature = "integration-tests")]` gate around integration tests (controlled by feature flag)

## Verification

- `cargo test -p connection` — at least 6 total tests (3 unit + 3 integration)
- Integration tests require Redis server on localhost:6379 (document this in tests)
- `cargo test -p connection --no-run` — compiles without errors even without Redis running
- `cargo clippy -p connection` — zero warnings
