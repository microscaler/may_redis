# Story 5.3 — InMemoryClient (feature=test)

**Objective:** Implement an in-memory Redis backend for test isolation.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.2

**Source docs:** `docs/10-test-strategy.md`, `docs/Epics/Epic_5/Story_0.md`

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

## Tasks

1. Define `InMemoryStore` with HashMap<String, (value, Option<TTL>)>
2. Define `InMemoryClient` wrapping `Arc<Mutex<InMemoryStore>>`
3. Implement all 11 methods above
4. TTL expiration — check TTL on GET/EXISTS/TTL, remove expired entries
5. EXPIRE — set TTL on existing key
6. KEYS — support `*` and `?*` glob patterns
7. INCR — atomic increment of string-to-i64, error on non-integer values
8. Gate all of this behind `#[cfg(feature = "test")]`
9. Re-export from umbrella crate: `may_redis::InMemoryClient` when test feature is enabled

## Verification

- `cargo test -p may-redis --features test` — at least 8 unit tests:
  - `test_inmemory_set_get` — set "key" "value", get returns "value"
  - `test_inmemory_set_ex_get` — set_ex "key" "value" 60, get returns "value"
  - `test_inmemory_del` — del returns 1 for existing key, 0 for missing
  - `test_inmemory_exists` — exists returns true for existing, false for missing
  - `test_inmemory_incr` — incr on non-existent returns 1, next incr returns 2
  - `test_inmemory_ttl` — ttl returns seconds for set_ex key
  - `test_inmemory_expire` — expire sets TTL, subsequent ttl reflects new TTL
  - `test_inmemory_keys_pattern` — keys "user:*" returns matching keys
  - `test_inmemory_flushdb` — flushdb clears all data, dbsize returns 0
- `cargo test -p may-redis --features test` — must pass without a running Redis server
- `cargo clippy -p may-redis --features test` — zero warnings
