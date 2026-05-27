# Migration Guide: `redis` → `may-redis`

**Version:** v0.1
**License:** MIT OR Apache-2.0

## Overview

`may-redis` is a coroutine-native Redis client built on the `may` runtime. It provides an API surface compatible with the `redis` crate for mechanical migration of Sesame-IDAM and other services.

This guide documents the differences between `redis` and `may-redis` and provides a step-by-step migration path.

---

## Dependency Change

**Before (`Cargo.toml`):**
```toml
[dependencies]
redis = "0.25"
```

**After (`Cargo.toml`):**
```toml
[dependencies]
may-redis = "0.1"
```

---

## Import Changes

### Block import

**Before:**
```rust
use redis::Commands;
```

**After:**
```rust
use may_redis::Commands;
```

### Types

| `redis` type | `may-redis` type | Notes |
|-------------|------------------|-------|
| `redis::Client` | `may_redis::RedisClient` | New `new()` constructor with URL |
| `redis::Cmd` | `may_redis::cmd()` | Builder function |
| `redis::Pipeline` | `may_redis::Pipeline` | Similar API |
| `redis::Error` | `may_redis::RedisError` | Different variant structure |
| `redis::Value` | `may_redis::RedisValue` | Different variant structure |
| `redis::Connection` | *(internal)* | Not exposed |

---

## Connection

### `redis`

```rust
let client = redis::Client::open("redis://127.0.0.1:6379")?;
let mut conn = client.get_connection()?;
```

### `may-redis`

```rust
use may_redis::RedisClient;

let client = RedisClient::connect("redis://127.0.0.1:6379").await?;
// No separate connection step — the client IS the connection
```

**Key difference:** `may-redis` uses a connection pool managed by a background epoll loop. There is no `get_connection()` — commands are sent directly through the `RedisClient`.

---

## Commands

### Basic command (GET)

**Before:**
```rust
let value: Option<String> = redis::cmd("GET")
    .arg("mykey")
    .query(&mut conn)?;
```

**After:**
```rust
let value: Option<String> = client.get("mykey")?;
```

### SET

**Before:**
```rust
let _: () = redis::cmd("SET")
    .arg("mykey")
    .arg("myvalue")
    .query(&mut conn)?;
```

**After:**
```rust
client.set("mykey", "myvalue")?;
```

### All supported commands

| `redis` method | `may-redis` method | Return type |
|---------------|-------------------|-------------|
| `get::<_, Option<String>>(key)` | `client.get(key)` | `Option<String>` |
| `set(key, value)` | `client.set(key, value)` | `()` |
| `set_ex(key, value, seconds)` | `client.set_ex(key, value, seconds)` | `()` |
| `del(keys)` | `client.del(keys)` | `usize` (keys deleted) |
| `exists(key)` | `client.exists(key)` | `bool` |
| `incr(key)` | `client.incr(key)` | `i64` |
| `ttl(key)` | `client.ttl(key)` | `i64` |
| `expire(key, seconds)` | `client.expire(key, seconds)` | `bool` |
| `publish(channel, message)` | `client.publish(channel, message)` | `i64` (subscribers) |
| `keys(pattern)` | `client.keys(pattern)` | `Vec<String>` |
| `dbsize()` | `client.dbsize()` | `usize` |
| `flushdb()` | `client.flushdb()` | `()` |
| `ping()` | `client.ping()` | `String` |
| `auth(password)` | `client.auth(password)` | `()` |

### Builder pattern

**Before:**
```rust
let cmd = redis::cmd("GET")
    .arg("mykey");
let result: Option<String> = cmd.query(&mut conn)?;
```

**After:**
```rust
use may_redis::cmd;
let builder = cmd("GET").arg("mykey");
let result: Option<String> = client.execute(builder)?;
```

---

## Pipelines

### `redis`

```rust
let mut pipe = redis::pipe();
pipe.cmd(redis::cmd("SET").arg("k1").arg("v1"));
pipe.cmd(redis::cmd("SET").arg("k2").arg("v2"));
pipe.cmd(redis::cmd("GET").arg("k1"));
let ((), (), value): ((), (), Option<String>) = pipe.query(&mut conn)?;
```

### `may-redis`

```rust
let mut pipe = client.pipeline();
pipe.add(client.set("k1", "v1"));
pipe.add(client.set("k2", "v2"));
pipe.add(client.get("k1"));
let ((), (), value): ((), (), Option<String>) = pipe.execute()?;
```

**Key difference:** `may-redis` uses the `Commands` trait methods (`client.set()`, `client.get()`) directly in `pipe.add()`, rather than building raw `Cmd` objects.

### Supported tuple sizes

`may-redis` implements `FromPipelineResponse` for tuples of 1 through 4 elements:

```rust
// 1 element
let (result1,): (Result1,) = pipe.execute()?;

// 2 elements
let (result1, result2): (Result1, Result2) = pipe.execute()?;

// 3 elements
let (result1, result2, result3): (Result1, Result2, Result3) = pipe.execute()?;

// 4 elements
let (r1, r2, r3, r4): (R1, R2, R3, R4) = pipe.execute()?;
```

For more than 4 results, use the builder pattern with `client.execute()` repeatedly.

---

## Type Mapping

### `redis::Value` → `may_redis::RedisValue`

| `redis::Value` | `may_redis::RedisValue` | Description |
|---------------|------------------------|-------------|
| `Value::Nil` | `RedisValue::Null` | Null/bulk-null |
| `Value::SimpleString(s)` | `RedisValue::SimpleString(s)` | Simple string |
| `Value::BulkString(v)` | `RedisValue::BulkString(v)` | Bulk string |
| `Value::Integer(n)` | `RedisValue::Integer(n)` | Integer |
| `Value::Array(a)` | `RedisValue::Array(a)` | Array |
| `Value::Error(s)` | `RedisValue::Error(s)` | Error |

### FromRedisValue type constraints

`may_redis` implements `FromRedisValue` for:

- `()` (for commands that return OK)
- `bool`
- `i64`
- `String`
- `Option<String>`
- `Vec<String>`
- `Vec<i64>`
- `Vec<RedisValue>`

**Not implemented:** `Vec<bool>`, `Vec<usize>`, arbitrary structs. Use the explicit types above.

---

## Error Handling

### `redis`

```rust
use redis::RedisError;

match client.get::<_, Option<String>>("key") {
    Ok(value) => println!("{:?}", value),
    Err(e) => println!("Error: {}", e),
}
```

### `may-redis`

```rust
use may_redis::RedisError;

match client.get("key") {
    Ok(value) => println!("{:?}", value),
    Err(e) => println!("Error: {e}"),
}
```

**Key difference:** All `execute()` and `Commands` methods return `Result<T, RedisError>`. Use `?` or explicit matching.

### Error types

- `RedisError::Parse(msg)` — Failed to convert server response to expected Rust type
- `RedisError::Connection(msg)` — Socket/connectivity failure
- `RedisError::Protocol(msg)` — RESP protocol violation
- `RedisError::Other(msg)` — Catch-all for other errors

---

## Coroutine Context

**Critical difference:** `may-redis` requires a `may` coroutine context for all operations. All `execute()` calls must run inside `may::go!`.

### Example in application code

```rust
may::go!({
    let client = RedisClient::connect("redis://127.0.0.1:6379").await.unwrap();
    client.set("key", "value").unwrap();
    let value: Option<String> = client.get("key").unwrap();
});
```

### Using in a synchronous context

If you need to call `may-redis` from a non-coroutine context, use `may::run`:

```rust
may::run(|| {
    may::go!({
        // Your may-redis code here
    }).join();
});
```

---

## Connection Pool

`may-redis` manages its own connection pool internally (single connection per client, backed by an epoll loop). You do not configure pool size.

For Sesame-IDAM use, this maps to one RedisClient per service instance, which is the recommended pattern.

---

## Migration Checklist

- [ ] Replace `redis = "..."` dependency with `may-redis = "0.1"`
- [ ] Replace `redis::Commands` import with `may_redis::Commands`
- [ ] Replace `redis::Client::open(url)` with `RedisClient::connect(url)`
- [ ] Replace `client.get_connection()` — no longer needed (use `client` directly)
- [ ] Replace `redis::cmd("...").arg(...).query(&mut conn)` with `client.commands(...)` or `cmd("...").arg(...)` + `client.execute()`
- [ ] Replace `redis::pipe().cmd(...)` with `client.pipeline().add(client.commands(...))`
- [ ] Verify all type annotations match `FromRedisValue` implementations (see Type Mapping section)
- [ ] Ensure all `execute()` calls run in `may::go!` coroutine context
- [ ] Run full test suite — all tests must pass

---

## Known Differences

### 1. No blocking operations

`may-redis` is fully coroutine-native. There are no blocking `Connection` operations — everything flows through the epoll loop. The `get_connection()` pattern does not exist.

### 2. RESP2 only

`may-redis` only supports RESP2 wire format (no RESP3 types like maps, attributes, or arbitrary binary).

### 3. Single connection model

`may-redis` uses a single connection per `RedisClient` with an internal request queue. The `redis` crate's `Client` with `get_connection()` creates new connections on demand. For `may-redis`, clone the `RedisClient` to share the same underlying connection across coroutines.

### 4. No raw command queries

`may-redis` does not have a `query()` method on `Cmd`. Use the `Commands` trait methods or `client.execute(builder)` for custom commands.

### 5. No `Transaction` type

`may-redis` does not implement Redis transactions (MULTI/EXEC). Pipeline ordering guarantees atomicity for most use cases.

### 6. Error type compatibility

`may_redis::RedisError` is NOT compatible with `redis::RedisError`. Error handling code must be updated.

---

## Verification

After migration:

1. `cargo test` — all tests pass
2. `cargo clippy` — no warnings
3. `cargo fmt --check` — clean
4. Manual smoke test against local Redis instance
5. Deploy to staging with monitoring
