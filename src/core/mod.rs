// core — Core types and traits for may-redis
//
// Provides the foundational types used across all other modules:
// - Value: The canonical representation of a Redis value
// - Error: Error representation matching Redis error protocol
// - FromValue: Trait for converting Value into Rust types
// - ToArgs: Trait for converting Rust types into Redis arguments

//! # core
//!
//! Core types: `Value`, `Error`, `FromValue`, `ToArgs`.
//!
//! This crate has zero dependency on `may` or any I/O runtime. It is pure data
//! and conversion logic.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph LR
//!     V[Value] --> E[Error]
//!     V --> FV[FromValue]
//!     V --> TA[ToArgs]
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use may_redis::RedisValue;
//!
//! let val = RedisValue::BulkString("hello".into());
//! assert!(matches!(val, RedisValue::BulkString(_)));
//! ```

pub mod error;
pub mod from_value;
pub mod to_args;
pub mod value;

// Re-export public types at the crate root for convenience
pub use error::{FromRedisValue, RedisError, RedisResult};
pub use value::RedisValue;
pub use to_args::ToRedisArgs;
