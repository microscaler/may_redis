# Story 16.2 — Wire fixture into integration tests

**Objective:** Replace the hardcoded `shared_client()` (which connects to `127.0.0.1:6379`) with fixture-managed connections in all integration test modules. Each test creates its own `RedisTestFixture`, uses the fixture's port, and drops it at test end for isolation.

**Epic:** 16 — Docker-Managed Test Fixtures

**Dependencies:** Story 16.1 (test_fixture.rs fixed)

**Status:** TODO

**Source docs:** `src/client/client_tests/unit.rs` (shared_client, run_may), `src/client/mod.rs` (mod declarations), `src/client/client_tests/integration_strings_basic.rs` (sample integration test file)

## Code Anchors

- `src/client/client_tests/unit.rs` — `shared_client()`, `run_may()` — needs modification
- `src/client/client_tests/*.rs` — all 10 integration test files — need modification
- `src/client/mod.rs` — module declarations for test modules

## Architecture Change

### Before (current)

```
test file
  → unit::shared_client()  // static OnceLock<RedisClient> → 127.0.0.1:6379
  → unit::run_may(|| { ... })
```

### After

```
test file
  → fixture = RedisTestFixture::builder()
                .with_plain_redis(true)
                .with_tls_redis(false)
                .build()
  → client = RedisClient::connect("127.0.0.1", fixture.host(0))
  → run_may(|| { ... })
  → drop(fixture)  // automatic cleanup via RAII
```

## Implementation Details

### 1. Modify `unit.rs` — add fixture-aware shared_client

The `shared_client()` function in `unit.rs` needs a fixture-aware variant:

```rust
// In unit.rs — add fixture helper
pub(super) fn shared_client_with_fixture() -> (RedisClient, test_fixture::RedisTestFixture) {
    let fixture = test_fixture::RedisTestFixture::builder()
        .with_plain_redis(true)
        .with_tls_redis(false)
        .build();
    let client = RedisClient::connect("127.0.0.1", fixture.host(0));
    (client, fixture)
}
```

The fixture must be returned as a tuple so the caller owns it and it drops at test end.

### 2. Update each integration test file

Every test in every integration test file needs to:

1. Create the fixture
2. Get the client with the fixture's port
3. Run the test logic
4. Drop the fixture (automatic via RAII)

Pattern for each test:

```rust
#[test]
#[ignore = "requires Docker + Redis container"]
fn test_strings_get() {
    let (client, _fixture) = shared_client_with_fixture();
    
    run_may(|| {
        client.execute::<()>(client.flushdb()).ok();
        // test logic
        client.execute::<()>(client.flushdb()).ok();
    });
    // _fixture dropped here, container auto-removed
}
```

The `_fixture` binding is underscore-prefixed because the drop happens automatically when it goes out of scope.

### 3. Files to modify

All 10 integration test files plus `unit.rs`:

| File | Tests | Port |
|------|-------|------|
| `unit.rs` | shared infrastructure | N/A (add fixture helper) |
| `integration_admin_basic.rs` | FLUSHDB, PING, INFO | fixture.host(0) |
| `integration_admin_advanced.rs` | CONFIG, DEBUG, SLOWLOG | fixture.host(0) |
| `integration_strings_basic.rs` | SET, GET, DEL, INCR, etc. | fixture.host(0) |
| `integration_strings_advanced.rs` | APPEND, MGET, MSET, EXISTS, etc. | fixture.host(0) |
| `integration_hashes_basic.rs` | HSET, HGET, HDEL, HMGET | fixture.host(0) |
| `integration_hashes_advanced.rs` | HSCAN, HVALS, HKEYS | fixture.host(0) |
| `integration_lists_basic.rs` | LPUSH, RPUSH, LPOP, RPOP, LRANGE | fixture.host(0) |
| `integration_sets_basic.rs` | SADD, SMEMBERS, SISMEMBER, SPOP | fixture.host(0) |
| `integration_sorted_sets.rs` | ZADD, ZRANGE, ZRANK, ZREM | fixture.host(0) |
| `integration_transactions.rs` | MULTI, EXEC, WATCH | fixture.host(0) |

### 4. Add `#[ignore]` attribute

All integration tests currently use `#[ignore = "requires live Redis server"]`. Change to:

```rust
#[ignore = "requires Docker + Redis container"]
```

This makes it clear the tests need Docker, not just any Redis.

## Tasks

- [ ] Add `test_fixture` module import path in `client/mod.rs` (or make it a separate crate-level module)
- [ ] Add `shared_client_with_fixture()` helper in `unit.rs` that creates fixture + client
- [ ] Update `integration_admin_basic.rs` — 2-3 tests, wire fixture
- [ ] Update `integration_admin_advanced.rs` — 3-4 tests, wire fixture
- [ ] Update `integration_strings_basic.rs` — ~15 tests, wire fixture
- [ ] Update `integration_strings_advanced.rs` — ~15 tests, wire fixture
- [ ] Update `integration_hashes_basic.rs` — ~10 tests, wire fixture
- [ ] Update `integration_hashes_advanced.rs` — ~8 tests, wire fixture
- [ ] Update `integration_lists_basic.rs` — ~12 tests, wire fixture
- [ ] Update `integration_sets_basic.rs` — ~12 tests, wire fixture
- [ ] Update `integration_sorted_sets.rs` — ~10 tests, wire fixture
- [ ] Update `integration_transactions.rs` — ~8 tests, wire fixture

## Acceptance Criteria

| # | AC | Maps to |
|---|----|---------|
| 1 | `shared_client_with_fixture()` returns `(RedisClient, RedisTestFixture)` tuple | FR-008 |
| 2 | Client connects to `127.0.0.1:{fixture.host(0)}` — no hardcoded port | FR-008, NFR-008 |
| 3 | All 10 integration test files use the fixture | FR-008 |
| 4 | No integration test hardcodes `127.0.0.1:6379` | NFR-007 |
| 5 | Each test has its own fixture — no shared mutable state between tests | FR-011 |
| 6 | Container is auto-removed when test drops fixture (RAII) | FR-003, NFR-002 |

## Verification

- [ ] `cargo check --tests` — compiles (no Docker needed)
- [ ] `cargo clippy --tests -- -D warnings` — clean
- [ ] With Docker: `cargo test --lib --features test -- --include-ignored test_integration_` — tests pass
- [ ] Without Docker: `SKIP_DOCKER_TESTS=1 cargo test --lib --features test` — all tests skip (not fail)
