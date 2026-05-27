# Codebase Entry Points

- Status: unverified
- Source docs: `src/lib.rs`, `src/*/mod.rs`
- Code anchors: 
  - `src/lib.rs` — root module, re-exports all sub-modules
  - `src/base/mod.rs` — RedisValue, RedisError, FromRedisValue, ToRedisArgs
  - `src/codec/mod.rs` — RESPWriter, RESPReader
  - `src/protocol/mod.rs` — CommandBuilder, Commands trait
  - `src/connection/mod.rs` — Connection, connection loop, TCP connector
  - `src/client/mod.rs` — RedisClient, Pipeline

## Root (`src/lib.rs`)

```rust
// may-redis — A coroutine-native Redis client for the may runtime
//
// Zero tokio, zero async-await, only may coroutines.
//
// Module layout:
// - base:        RedisValue, RedisError, FromRedisValue, ToRedisArgs
// - codec:       RESP encoding/decoding (writer + reader)
// - protocol:    CommandBuilder, Commands trait
// - connection:  epoll connection loop, TCP, coroutine management
// - client:      RedisClient, Pipeline, public API

pub mod base;
pub mod codec;
pub mod protocol;
pub mod connection;
pub mod client;
```

## Module Entry Points

| Module | Primary Types | Key Functions |
|--------|--------------|---------------|
| `base` | `RedisValue`, `RedisError`, `FromRedisValue`, `ToRedisArgs` | Type conversions, trait impls |
| `codec` | `RESPWriter`, `RESPReader` | `encode_command()`, `decode_response()` |
| `protocol` | `CommandBuilder`, `Commands` | `cmd()`, `commands` trait methods |
| `connection` | `Connection`, `Request` | `connect()`, `send()`, `receive()` |
| `client` | `RedisClient`, `Pipeline` | `RedisClient::connect()`, `execute()`, `pipeline()` |
