# Story 5.3 — InMemoryClient (feature=test)

**Objective:** Implement an in-memory Redis backend for test isolation.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.2 (Pipeline API)

**Status:** PARTIAL — core implementation exists but has two gaps:

1. **Missing `#[cfg(feature = "test")]` gate** — `in_memory.rs` is always compiled, not gated behind the feature flag despite `test = []` existing in `Cargo.toml`
2. **API mismatch** — methods return bare values (`bool`, `usize`) instead of `Result<T, RedisError>` as specified in the NFRs, breaking API consistency with the real `RedisClient`

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_5/Story_0.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** `InMemoryStore` stores key-value pairs with optional TTL expiry
- [x] **FR-2:** `InMemoryClient::get(key)` returns stored value or error if missing/expired
- [x] **FR-3:** `InMemoryClient::set(key, value)` stores a value
- [x] **FR-4:** `InMemoryClient::set_ex(key, value, seconds)` stores a value with TTL
- [x] **FR-5:** `InMemoryClient::del(key)` removes a key, returns number of keys deleted (0 or 1)
- [x] **FR-6:** `InMemoryClient::exists(key)` returns true if key exists and not expired
- [x] **FR-7:** `InMemoryClient::incr(key)` atomically increments integer value, returns new value
- [x] **FR-8:** `InMemoryClient::ttl(key)` returns remaining TTL in seconds, or error
- [x] **FR-9:** `InMemoryClient::expire(key, seconds)` sets TTL on existing key
- [x] **FR-10:** `InMemoryClient::keys(pattern)` returns matching keys with glob support
- [x] **FR-11:** `InMemoryClient::dbsize()` returns number of keys in store
- [x] **FR-12:** `InMemoryClient::flushdb()` clears all data
- [x] **FR-13:** TTL expiration is checked lazily on GET/EXISTS/TTL access
- [ ] **FR-14:** All of this is gated behind `#[cfg(feature = "test")]` — **GAP**
- [x] **FR-15:** Re-exported from umbrella crate: `may_redis::InMemoryClient` when test feature enabled

### Non-Functional Requirements

- [x] **NFR-1:** No `may` runtime dependency — pure in-memory store
- [x] **NFR-2:** Thread-safe: `InMemoryClient` is `Send + Sync` (protected by `Mutex`)
- [x] **NFR-3:** No dependency on a running Redis server for tests
- [x] **NFR-4:** TTL uses `std::time::Instant` for expiration checks
- [x] **NFR-5:** INCR on non-integer value returns error (not crash)
- [x] **NFR-6:** INCR on missing key returns 1 (auto-create)
- [x] **NFR-7:** KEYS supports `*` (match all) and `?*` (single char wildcard) glob patterns

## Gaps

### Gap 1: Missing feature flag gate

`src/client/in_memory.rs` is compiled unconditionally. The `test = []` feature exists in `Cargo.toml` but the module is not gated:

```rust
// Current (wrong): always compiled
pub mod in_memory;

// Should be:
#[cfg(feature = "test")]
pub mod in_memory;
```

### Gap 2: API returns bare values instead of Result<T, RedisError>

The stories specify that `InMemoryClient` methods should match the `RedisClient` API surface and return `Result<T, RedisError>`. Current implementation returns bare values:

```rust
// Current (wrong):
pub fn del(&self, key: &str) -> usize
pub fn exists(&self, key: &str) -> bool

// Should be:
pub fn del(&self, key: &str) -> Result<usize, RedisError>
pub fn exists(&self, key: &str) -> Result<bool, RedisError>
```

This matters because integration tests use the same client type pattern and the inconsistency breaks abstraction.

## Code Anchors

- `src/client/mod.rs` — conditional: `#[cfg(feature = "test")] pub mod in_memory;`
- `src/client/in_memory.rs` — InMemoryClient + InMemoryStore
- `Cargo.toml` — `test = []` feature flag

## Structs

```rust
pub struct InMemoryStore {
    data: HashMap<String, (String, Option<Instant>)>,
}

pub struct InMemoryClient {
    store: Arc<Mutex<InMemoryStore>>,
}
```

## Implementation Tasks

- [x] Define `InMemoryStore` with `HashMap<String, (String, Option<Instant>)>`
- [x] Define `InMemoryClient` wrapping `Arc<Mutex<InMemoryStore>>`
- [x] Implement `get(&self, key: &str)` — check TTL, return value or error if missing
- [x] Implement `set(&self, key: &str, value: &str)` — stores value
- [x] Implement `set_ex(&self, key: &str, value: &str, seconds: u64)` — stores with TTL
- [x] Implement `del(&self, key: &str)` — removes key
- [x] Implement `exists(&self, key: &str)` — returns bool
- [x] Implement `incr(&self, key: &str)` — increments integer value
- [x] Implement `ttl(&self, key: &str)` — returns TTL seconds
- [x] Implement `expire(&self, key: &str, seconds: u64)` — sets TTL
- [x] Implement `keys(&self, pattern: &str)` — glob match
- [x] Implement `dbsize(&self)` — returns count
- [x] Implement `flushdb(&self)` — clears all data
- [x] Re-export from umbrella crate: `may_redis::InMemoryClient`
- [ ] **GAP:** Gate all implementations behind `#[cfg(feature = "test")]`
- [ ] **GAP:** Change method signatures to return `Result<T, RedisError>` consistently

## Verification

### Unit Tests (minimum 9)

All 11 methods exist and function correctly, but tests need feature flag gating:

- `test_inmemory_set_get` — set "key" "value", get returns "value"
- `test_inmemory_set_ex_get` — set_ex "key" "value" 60, get returns "value"
- `test_inmemory_del` — del returns 1 for existing key, 0 for missing
- `test_inmemory_exists` — exists returns true for existing, false for missing
- `test_inmemory_incr` — incr on non-existent returns 1, next incr returns 2
- `test_inmemory_ttl` — ttl returns seconds for set_ex key
- `test_inmemory_expire` — expire sets TTL, subsequent ttl reflects new TTL
- `test_inmemory_keys_pattern` — keys "user:*" returns matching keys
- `test_inmemory_flushdb` — flushdb clears all data, dbsize returns 0

- `cargo clippy` — zero warnings
- **Gap:** Feature-gated test suite not yet configured
