# Sesame-IDAM Redis Usage Analysis

## Current Usage Overview

Sesame-IDAM uses Redis in **4 distinct modules** across `common/` and individual microservices.
All currently depend on the `redis` crate with `tokio-comp` feature, creating a hard dependency
on tokio where only `may` coroutines should run.

## Usage Inventory

```mermaid
graph TB
    subgraph "common/"
        D[denylist/\n cache.rs]
        E[entitlement_cache/\n mod.rs]
        F[fallback_cache/\n mod.rs]
        J[jwt_common_path/\n dpop.rs\n shadow_decision.rs]
        TV[token_versioning/\n publisher.rs\n subscriber.rs\n version_store.rs]
    end
    
    subgraph "microservices/"
        IDAM[identity-session/
            service.rs]
        LOGIN[identity-login/\n social_oauth.rs\n otp_service.rs]
    end
    
    D --> "PUBLISH/KEYS/GET"
    E --> "GET/SET/EXISTS/EXPIRE"
    F --> "GET/SET/EXISTS/DBSIZE/FLUSHDB"
    J --> "SET/GET/EXISTS"
    TV --> "INCR/EXPIRE/GET/DEL/PUBLISH"
    
    IDAM --> "SET/GET"
    LOGIN --> "SET/GET"
```

## Module-by-Module Analysis

### 1. DPoP Proof Store (`jwt_common_path/dpop.rs`)

```mermaid
graph LR
    subgraph "Production"
        Store[RedisProofStore]
        Init[init url\n get_multiplexed_async_connection]
        IsSeen[is_seen jti\n EXISTS dpop_jti:{jti}]
        Record[record jti\n SET dpop_jti:{jti} seen EX 60]
    end
    
    subgraph "Interface"
        Trait[DpopProofStore]
        Mem[InMemoryProofStore]
    end
    
    Trait -.implements.-> Store
    Trait -.implements.-> Mem
    Store --> Init
    Store --> IsSeen
    Store --> Record
```

**Commands:** `EXISTS`, `SET EX 60`
**Pattern:** Request-response, single connection, multiplexed
**Current tokio deps:**
- `tokio::sync::Mutex` — protects the stored connection
- `redis::aio::MultiplexedConnection` — async connection
- `redis::cmd().query_async()` — async command execution

### 2. Fallback Cache (`jwt_common_path/fallback_cache.rs`)

```mermaid
graph TB
    subgraph "Production Path"
        GetCache[get_from_cache]\n [GET]\n [SET EX]
        GetDb[get_from_db]\n [GET on cache miss]
    end
    
    subgraph "Test Path (synchronous)"
        TestDbSize[test_cache_config_defaults\n DBSIZE]\n TestFlush[test_clear_removes_all_entries\n FLUSHDB]
        TestKeys[test_lru_eviction\n KEYS pattern]
    end
    
    GetCache --> Redis
    GetDb --> Redis
    Redis --> "Redis Server"
```

**Commands:** `GET`, `SET`, `DBSIZE`, `FLUSHDB`, `KEYS`
**Pattern:** Test code uses synchronous `get_connection()`; prod uses `redis::Commands` trait
**Current tokio deps:** None (synchronous API in current impl, but Cargo.toml requires tokio-comp)

### 3. Token Versioning (`token_versioning/`)

```mermaid
graph TB
    subgraph "Publisher"
        Pub[publisher.rs\n PUBLISH version_channel]
    end
    
    subgraph "Subscriber"
        Sub[subscriber.rs\n Subscribe on channel\n GET versions]
    end
    
    subgraph "Version Store"
        Vs[version_store.rs\n INCR/EXPIRE/GET/DEL/TTL]
    end
    
    Pub --> Redis
    Sub --> Redis
    Vs --> Redis
    Redis --> "Redis Server"
```

**Commands:** `PUBLISH`, `KEYS`, `GET`, `INCR`, `EXPIRE`, `DEL`, `TTL`
**Pattern:** Pub/sub + version store, multiplexed connection
**Current tokio deps:**
- `redis::aio::MultiplexedConnection`
- `redis::cmd().query_async()`

### 4. Denylist Cache (`denylist/cache.rs`)

**Commands:** `GET`, `SET EX`
**Pattern:** Simple key-value cache
**Current tokio deps:** Same as dpop.rs

### 5. Entitlement Cache (`entitlement_cache/mod.rs`)

**Commands:** `GET`, `SET`, `DEL`, `TTL`
**Pattern:** LRU cache with Redis backing
**Current tokio deps:** Same pattern

## Command Set Summary

| Command | Used By | Frequency | Notes |
|---------|---------|-----------|-------|
| `SET key val EX seconds` | dpop, fallback, denylist | HIGH | Core cache primitive |
| `GET key` | fallback, denylist, version_store, entitlement | HIGH | Read path |
| `EXISTS key` | dpop | LOW | Check before set |
| `INCR key` | version_store | LOW | Atomic counter |
| `DEL key` | version_store, entitlement | LOW | Cleanup |
| `TTL key` | version_store, entitlement | LOW | Check expiry |
| `EXPIRE key seconds` | version_store | LOW | Set TTL after set |
| `PUBLISH channel message` | token_versioning/publisher | MEDIUM | Pub/sub |
| `KEYS pattern` | tests, subscriber | LOW | Debug/scan (slow!) |
| `DBSIZE` | tests | LOW | Monitor |
| `FLUSHDB` | tests | LOW | Cleanup |
| `AUTH password` | tests | N/A | Not used yet |

## Conclusion

**This is a very small command set.** We only need ~10 command primitives to cover all
current and future use cases. The RESP encoding/decoding for these is straightforward:
- All commands are bulk arrays
- All responses are either simple strings, integers, bulk strings, arrays, or errors

