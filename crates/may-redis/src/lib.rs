// may-redis — Umbrella re-export crate
//
// Re-exports all public types from the workspace crates for convenient
// single-crate usage by downstream consumers.

//! # may-redis
//!
//! A coroutine-native Redis client for the [may](https://crates.io/crates/may) runtime.
//!
//! Zero tokio, zero async-await, only may coroutines.
//!
//! ## Workspace Structure
//!
//! ```mermaid
//! graph LR
//!     subgraph "Workspace: may_redis"
//!         M[may-redis] --> C[client]
//!         C --> CO[connection]
//!         C --> P[protocol]
//!         P --> CO2[codec]
//!         CO --> CO2
//!         P --> B[base]
//!         CO2 --> B
//!         CO --> B
//!     end
//! ```
//!
//! ## Feature Flags
//!
//! - `default`: `["connection", "client"]`
//! - `connection`: Enable TCP connection support
//! - `client`: Enable `RedisClient` and `Pipeline` (requires `connection`)
//! - `pool`: Connection pooling (future)
//! - `test`: Test helpers and `InMemoryClient`
//!
//! ## Quick Start
//!
//! ```ignore
//! use may_redis::{RedisClient, Commands};
//!
//! // In a may coroutine context:
//! let mut client = RedisClient::connect("127.0.0.1:6379").await?;
//! client.set("key", "value").await?;
//! let val: Option<String> = client.get("key").await?;
//! ```

#[cfg(feature = "connection")]
pub use connection;

#[cfg(feature = "client")]
pub use client;

pub use base;
pub use codec;
pub use protocol;
