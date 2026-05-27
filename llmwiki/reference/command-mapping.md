# Command Mapping: redis → may-redis

- Status: unverified
- Source docs: `docs/07-client-api-design.md`, `docs/09-migration-guide.md`, `docs/03-sesame-idam-redis-usage.md`
- Code anchors: `src/protocol/`, `src/client/`
- Last updated: 2026-05-27

## Commands Trait (may-redis)

| may-redis Method | redis Equivalent | Args | Returns |
|------------------|-----------------|------|---------|
| `get(key)` | `client.get(key)` | `K: ToRedisArgs` | `V: FromRedisValue` |
| `set(key, value)` | `client.set(key, value)` | `K, V: ToRedisArgs` | `()` |
| `set_ex(key, value, seconds)` | `client.set_ex(key, value, seconds)` | `K, V: ToRedisArgs; u32` | `()` |
| `exists(key)` | `client.exists(key)` | `K: ToRedisArgs` | `usize` |
| `del(key)` | `client.del(key)` | `K: ToRedisArgs` | `usize` |
| `incr(key)` | `client.incr(key)` | `K: ToRedisArgs` | `i64` |
| `ttl(key)` | `client.ttl(key)` | `K: ToRedisArgs` | `i64` |
| `expire(key, seconds)` | `client.expire(key, seconds)` | `K: ToRedisArgs; u32` | `bool` |
| `publish(channel, message)` | `client.publish(channel, message)` | `K, M: ToRedisArgs` | `usize` |
| `keys(pattern)` | `client.keys(pattern)` | `K: ToRedisArgs` | `Vec<String>` |
| `dbsize()` | `client.dbsize()` | — | `usize` |
| `flushdb()` | `client.flushdb()` | — | `()` |
| `ping()` | `client.ping()` | — | `String` |
| `auth(password)` | `client.auth(password)` | `&str` | `String` |

## Raw Command Builder

```rust
// Build and execute raw commands
let cmd = cmd("SET").arg("key").arg("value").arg("EX").arg(60u32);
let result: Result<(), RedisError> = client.execute(cmd).await?;

// Or directly:
let resp: Result<String, RedisError> = client.execute(cmd("GET").arg("key")).await?;
```

## Migration Mapping (redis → may-redis)

| Before (redis) | After (may-redis) | Notes |
|----------------|-------------------|-------|
| `redis::Client::open(url)?.get_multiplexed_async_connection().await?` | `RedisClient::connect(url)?` | Synchronous, may-aware |
| `redis::cmd("GET").arg(&key).query_async(conn).await?` | `client.execute(cmd("GET").arg(&key)).await?` | No connection parameter |
| `use redis::aio::MultiplexedConnection` | *(remove)* | Connection is opaque |
| `tokio::sync::Mutex<Connection>` | *(remove)* | may-redis handles concurrency |
| `redis::cmd("FLUSHDB").query(&mut con)?` | `InMemoryClient` (feature=test) | Test code uses InMemoryClient |

## Sesame-IDAM File-by-File Migration

| File | Changes | Complexity |
|------|---------|------------|
| `jwt_common_path/dpop.rs` | Replace Client, remove Mutex, change .await | Medium |
| `jwt_common_path/fallback_cache.rs` | Replace sync .query() calls | Easy |
| `token_versioning/publisher.rs` | Replace PUBLISH command | Easy |
| `token_versioning/subscriber.rs` | Replace MultiplexedConnection, .await | Medium |
| `token_versioning/version_store.rs` | Replace all query_async calls | Medium |
| `common/denylist/cache.rs` | Replace if still using it | Easy |
| `common/entitlement_cache/mod.rs` | Replace .block_on() test issues | Medium |
