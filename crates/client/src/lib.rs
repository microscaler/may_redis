// client — Public client API
//
// Provides the user-facing API:
// - RedisClient: main entry point for all Redis operations
// - Pipeline: batch command execution
// - Mirrors the redis crate API surface for mechanical migration

//! # client
//!
//! Public client API: `RedisClient`, `Pipeline`.
//!
//! This crate assembles all lower-level crates: base, codec, protocol, connection.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph TB
//!     RC[RedisClient] --> CT[Commands trait]
//!     RC --> PJ[Pipeline]
//!     RC --> CO[Connection]
//!     CO --> Proto[protocol]
//!     CO --> Codec[codec]
//!     CO --> Base[base]
//!     PJ --> Proto
//!     PJ --> Codec
//!     PJ --> Base
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use client::RedisClient;
//!
//! let mut client = RedisClient::connect("127.0.0.1:6379").await?;
//! let val: Option<String> = client.get("mykey").await?;
//! ```

pub mod pipeline;
