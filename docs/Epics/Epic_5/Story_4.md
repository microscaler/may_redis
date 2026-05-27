# Story 5.4 — Integration tests and end-to-end verification

**Objective:** Full end-to-end integration tests connecting to a real Redis server and verifying the complete client pipeline works correctly.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.3 (InMemoryClient)

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
- [ ] Implement `test_integration_set_get` — SET + GET roundtrip
- [ ] Implement `test_integration_incr` — INCR sequence
- [ ] Implement `test_integration_del` — DEL existing and missing keys
- [ ] Implement `test_integration_exists` — EXISTS for present and absent keys
- [ ] Implement `test_integration_ttl_expire` — TTL set and expire
- [ ] Implement `test_integration_pipeline` — multiple commands in pipeline
- [ ] Implement `test_integration_concurrent` — multiple coroutines sharing one client
- [ ] Gate all integration tests with `#[ignore]` attribute
- [ ] Document how to run: `cargo test -- --ignored` with Redis running

## Verification

### Integration Tests (minimum 5, marked `#[ignore]`)

- [ ] `test_integration_set_get`
- [ ] `test_integration_incr`
- [ ] `test_integration_del`
- [ ] `test_integration_exists`
- [ ] `test_integration_ttl_expire`
- [ ] `test_integration_pipeline`
- [ ] `test_integration_concurrent`

### Commands

```bash
# With Redis running on localhost:6379:
cargo test -- --ignored

# Without Redis (only unit tests):
cargo test
```

### Lint & Build

- [ ] `cargo test --workspace` — all unit tests pass
- [ ] `cargo test -- --ignored` — all integration tests pass (with Redis)
- [ ] `cargo clippy --workspace --all-targets --all-features` — zero warnings
- [ ] `cargo fmt --workspace` — formatted
