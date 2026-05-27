# Story 5.3 — InMemoryClient (feature=test)

**Objective:** Implement an in-memory Redis backend for test isolation.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.2 (Pipeline API)

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_5/Story_0.md`

## Requirements

### Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| FR-1 | `InMemoryStore` stores key-value pairs with optional TTL expiry | P0 |
| FR-2 | `InMemoryClient::get(key)` returns stored value or error if missing/expired | P0 |
| FR-3 | `InMemoryClient::set(key, value)` stores a value | P0 |
| FR-4 | `InMemoryClient::set_ex(key, value, seconds)` stores a value with TTL | P0 |
| FR-5 | `InMemoryClient::del(key)` removes a key, returns number of keys deleted (0 or 1) | P0 |
| FR-6 | `InMemoryClient::exists(key)` returns true if key exists and not expired | P0 |
| FR-7 | `InMemoryClient::incr(key)` atomically increments integer value, returns new value | P0 |
| FR-8 | `InMemoryClient::ttl(key)` returns remaining TTL in seconds, or error | P0 |
| FR-9 | `InMemoryClient::expire(key, seconds)` sets TTL on existing key | P1 |
| FR-10 | `InMemoryClient::keys(pattern)` returns matching keys with glob support | P1 |
| FR-11 | `InMemoryClient::dbsize()` returns number of keys in store | P1 |
| FR-12 | `InMemoryClient::flushdb()` clears all data | P1 |
| FR-13 | TTL expiration is checked lazily on GET/EXISTS/TTL access | P0 |
| FR-14 | All of this is gated behind `#[cfg(feature = "test")]` | P0 |
| FR-15 | Re-exported from umbrella crate: `may_redis::InMemoryClient` when test feature enabled | P1 |

### Non-Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| NFR-1 | No `may` runtime dependency — pure in-memory store | P0 |
| NFR-2 | Thread-safe: `InMemoryClient` must be `Send + Sync` (protected by `Mutex`) | P0 |
| NFR-3 | No dependency on a running Redis server for tests | P0 |
| NFR-4 | TTL uses `std::time::Instant` for expiration checks | P0 |
| NFR-5 | INCR on non-integer value returns error (not crash) | P1 |
| NFR-6 | INCR on missing key returns 1 (auto-create) | P1 |
| NFR-7 | KEYS supports `*` (match all) and `?*` (single char wildcard) glob patterns | P2 |

## Code Anchors

- `crates/client/src/lib.rs` — conditional: `#[cfg(feature = "test")] pub mod in_memory;`
- `crates/client/src/in_memory.rs` — InMemoryClient + InMemoryStore
- `crates/may-redis/Cargo.toml` — `test = []` feature flag

## InMemory Store Diagram

```mermaid
graph TB
    subgraph "InMemoryStore (no may dependency)"
        Data[HashMap<String, (value, Option<TTL>)>]
        IC[InMemoryClient<br/>Arc<Mutex<InMemoryStore>>]
        
        IC --> Data
    end
    
    subgraph "Methods"
        Get[get key → value or error]
        Set[set key value]
        SetEx[set_ex key value seconds]
        Del[del key → count]
        Exists[exists key → bool]
        Incr[incr key → i64]
        Ttl[ttl key → seconds]
        Expire[expire key seconds → bool]
        Keys[keys pattern → Vec<String>]
        Dbsize[dbsize → usize]
        Flushdb[flushdb → ()]
    end
    
    IC --> Get
    IC --> Set
    IC --> SetEx
    IC --> Del
    IC --> Exists
    IC --> Incr
    IC --> Ttl
    IC --> Expire
    IC --> Keys
    IC --> Dbsize
    IC --> Flushdb
```

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

- [ ] Define `InMemoryStore` with `HashMap<String, (String, Option<Instant>)>`
- [ ] Define `InMemoryClient` wrapping `Arc<Mutex<InMemoryStore>>`
- [ ] Implement `get(&self, key: &str) -> Result<String, RedisError>`:
  - [ ] Lock mutex
  - [ ] Check TTL, remove expired entries
  - [ ] Return value or error if missing
- [ ] Implement `set(&self, key: &str, value: &str) -> Result<(), RedisError>`
- [ ] Implement `set_ex(&self, key: &str, value: &str, seconds: u64) -> Result<(), RedisError>`
- [ ] Implement `del(&self, key: &str) -> Result<usize, RedisError>` — returns 0 or 1
- [ ] Implement `exists(&self, key: &str) -> Result<bool, RedisError>`
- [ ] Implement `incr(&self, key: &str) -> Result<i64, RedisError>`:
  - [ ] Parse current value as i64
  - [ ] Increment and store
  - [ ] Return new value
- [ ] Implement `ttl(&self, key: &str) -> Result<u64, RedisError>`
- [ ] Implement `expire(&self, key: &str, seconds: u64) -> Result<bool, RedisError>`
- [ ] Implement `keys(&self, pattern: &str) -> Result<Vec<String>, RedisError>`:
  - [ ] Support `*` (match all)
  - [ ] Support `?*` (single char wildcard)
- [ ] Implement `dbsize(&self) -> Result<usize, RedisError>`
- [ ] Implement `flushdb(&self) -> Result<(), RedisError>`
- [ ] Gate all implementations behind `#[cfg(feature = "test")]`
- [ ] Re-export from umbrella crate: `may_redis::InMemoryClient` when test feature enabled
- [ ] Update `crates/may-redis/Cargo.toml` with `test = []` feature

## Verification

### Unit Tests (minimum 9)

- [ ] `test_inmemory_set_get` — set "key" "value", get returns "value"
- [ ] `test_inmemory_set_ex_get` — set_ex "key" "value" 60, get returns "value"
- [ ] `test_inmemory_del` — del returns 1 for existing key, 0 for missing
- [ ] `test_inmemory_exists` — exists returns true for existing, false for missing
- [ ] `test_inmemory_incr` — incr on non-existent returns 1, next incr returns 2
- [ ] `test_inmemory_ttl` — ttl returns seconds for set_ex key
- [ ] `test_inmemory_expire` — expire sets TTL, subsequent ttl reflects new TTL
- [ ] `test_inmemory_keys_pattern` — keys "user:*" returns matching keys
- [ ] `test_inmemory_flushdb` — flushdb clears all data, dbsize returns 0

### Integration Tests

- [ ] `cargo test -p may-redis --features test` — all tests pass without a running Redis server
- [ ] `cargo clippy -p may-redis --features test` — zero warnings
- [ ] `cargo fmt -p may-redis` — formatted
