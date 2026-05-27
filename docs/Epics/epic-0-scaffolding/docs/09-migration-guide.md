# Migration Guide: redis → may-redis

## Overview

This guide covers the migration from the `redis` crate to `may-redis` in sesame-idam.
The goal is minimal code changes — a mechanical search/replace where possible.

## Migration Path

### Phase 1: Drop tokio-comp Feature

```diff
# In Cargo.toml
redis = { workspace = true, features = ["aio", "connection-manager"] }
# Remove "tokio-comp" feature
```

### Phase 2: Replace Imports

Search and replace across all files:
```diff
-use redis::Client;
+use may-redis::Client;

-use redis::aio::MultiplexedConnection;
+// No longer needed — Connection is opaque
```

### Phase 3: Replace Connection Pattern

The biggest change: no more `get_multiplexed_async_connection().await`.
Connection is now synchronous (coroutine-friendly).

```diff
// Before (tokio-redis)
pub async fn init(&mut self, redis_url: &str) -> Result<(), DpopError> {
    let conn = redis::Client::open(redis_url)
        .map_err(|e| DpopError::InvalidJwk(format!("Redis: {e}")))?
        .get_multiplexed_async_connection()
        .await  // ← tokio .await
        .map_err(|e| DpopError::InvalidJwk(format!("Redis: {e}")))?;
    let mut guard = self.conn.lock().await;  // ← tokio Mutex
    *guard = Some(conn);
    Ok(())
}

// After (may-redis)
pub fn init(&mut self, redis_url: &str) -> Result<(), DpopError> {
    let client = Client::connect(redis_url)
        .map_err(|e| DpopError::InvalidJwk(format!("Redis: {e}")))?;
    self.client = Some(client);
    Ok(())
}
```

### Phase 4: Replace .query_async().await

```diff
// Before
let exists: i64 = redis::cmd("EXISTS")
    .arg(&key)
    .query_async::<_, i64>(conn)
    .await  // ← tokio .await
    .map_err(...)?;

// After
let exists: i64 = client.execute(
    cmd("EXISTS").arg(&key)
).await.map_err(...)?;
```

Or with the Commands trait (if it provides `exists()`):
```diff
let exists: i64 = client.exists(&key).await.map_err(...)?;
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
// Before (synchronous, uses .query())
let mut con = redis::Client::open(self.redis_url)?.get_connection()?;
let _: () = redis::cmd("FLUSHDB").query(&mut con)?;

// After (use InMemoryClient for tests, or synchronous wrapper)
use may-redis::testing::InMemoryClient;
let mut client = InMemoryClient::new();
client.set("key", "value");
assert_eq!(client.get::<String>("key").unwrap(), "value");
```

## File-by-File Migration Map

| File | Changes Needed | Complexity |
|------|---------------|------------|
| `dpop.rs` | Replace Client, remove Mutex, change .await | Medium |
| `fallback_cache.rs` | Replace sync .query() calls | Easy |
| `publisher.rs` | Replace PUBLISH command | Easy |
| `subscriber.rs` | Replace MultiplexedConnection, .await | Medium |
| `version_store.rs` | Replace all query_async calls | Medium |
| `denylist/cache.rs` | Replace if still using it | Easy |
| `entitlement_cache/mod.rs` | Replace .block_on() test issues | Medium |

## Expected Token Savings

| Token | Before | After | Delta |
|-------|--------|-------|-------|
| tokio dep | Full tokio crate (~800KB) | may-redis (~2KB) | -798KB |
| async state machines | Generated for each .await | None | Simplified |
| Connection pool code | redis connection-manager | may-redis (no pool needed) | Removed |
