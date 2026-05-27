# Sesame-IDAM Integration

- Status: unverified
- Source docs: `docs/03-sesame-idam-redis-usage.md`, `docs/09-migration-guide.md`
- Code anchors: `microscaler/sesame-idam/` (sibling repo)
- Last updated: 2026-05-27

## Current Sesame-IDAM Redis Usage

Sesame-IDAM uses Redis in **5 distinct modules** across `common/` and individual microservices. All currently depend on the `redis` crate with `tokio-comp` feature, creating a hard dependency on tokio where only `may` coroutines should run.

### Usage Inventory

| Module | Commands | Pattern |
|--------|----------|---------|
| `jwt_common_path/dpop.rs` | EXISTS, SET EX 60 | Request-response, single connection, multiplexed |
| `jwt_common_path/fallback_cache.rs` | GET, SET, DBSIZE, FLUSHDB, KEYS | Test uses sync; prod uses redis::Commands trait |
| `token_versioning/publisher.rs` | PUBLISH, KEYS | Pub/sub |
| `token_versioning/subscriber.rs` | SUBSCRIBE, GET | Pub/sub + version reads |
| `token_versioning/version_store.rs` | INCR, EXPIRE, GET, DEL, TTL | Atomic counter + expiry |
| `common/denylist/cache.rs` | GET, SET EX | Simple key-value cache |
| `common/entitlement_cache/mod.rs` | GET, SET, DEL, TTL | LRU cache with Redis backing |
| `identity-session/service.rs` | SET, GET | Session storage |
| `identity-login/social_oauth.rs` | SET, GET | OAuth state |
| `identity-login/otp_service.rs` | SET, GET | OTP storage |

### Command Set Summary

| Command | Used By | Frequency |
|---------|---------|-----------|
| `SET key val EX seconds` | dpop, fallback, denylist | HIGH |
| `GET key` | fallback, denylist, version_store, entitlement | HIGH |
| `EXISTS key` | dpop | LOW |
| `INCR key` | version_store | LOW |
| `DEL key` | version_store, entitlement | LOW |
| `TTL key` | version_store, entitlement | LOW |
| `EXPIRE key seconds` | version_store | LOW |
| `PUBLISH channel message` | token_versioning/publisher | MEDIUM |
| `KEYS pattern` | tests, subscriber | LOW |
| `DBSIZE` | tests | LOW |
| `FLUSHDB` | tests | LOW |
| `AUTH password` | tests | N/A |

**This is a very small command set** — only ~10 command primitives needed. All commands are bulk arrays, all responses are simple strings, integers, bulk strings, arrays, or errors.

## Migration Phases

### Phase 1: Drop tokio-comp Feature

```diff
# In Cargo.toml
-redis = { workspace = true, features = ["aio", "connection-manager", "tokio-comp"] }
+may-redis = { workspace = true }
```

### Phase 2: Replace Imports

```diff
-use redis::Client;
+use may_redis::Client;
```

### Phase 3: Replace Connection Pattern

Biggest change: no more `get_multiplexed_async_connection().await`. Connection is synchronous (coroutine-friendly).

### Phase 4: Replace .query_async().await

```diff
-let exists: i64 = redis::cmd("EXISTS").arg(&key).query_async::<_, i64>(conn).await?;
+let exists: i64 = client.execute(cmd("EXISTS").arg(&key)).await?;
```

### Phase 5: Replace tokio::sync::Mutex

```diff
-use tokio::sync::Mutex<Option<Connection>>;
+// Store client directly, no mutex needed
+// may-redis handles concurrency internally
+client: Option<Client>,
```

### Phase 6: Fix Test Code

```diff
-use redis::Client::open(...)?.get_connection()?;
+use may_redis::testing::InMemoryClient;
+let mut client = InMemoryClient::new();
```

## Expected Token Savings

| Token | Before | After | Delta |
|-------|--------|-------|-------|
| tokio dep | Full tokio crate (~800KB) | may-redis (~2KB) | -798KB |
| async state machines | Generated for each .await | None | Simplified |
| Connection pool code | redis connection-manager | may-redis (no pool needed) | Removed |
