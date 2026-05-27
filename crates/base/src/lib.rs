// base — Core Redis types for may-redis
//
// Provides the foundational types used across all other crates:
// - RedisValue: The canonical representation of a Redis value
// - RedisError: Error representation matching Redis error protocol
// - FromRedisValue: Trait for converting RedisValue into Rust types
// - ToRedisArgs: Trait for converting Rust types into Redis arguments

//! # base
//!
//! Core Redis types: `RedisValue`, `RedisError`, `FromRedisValue`, `ToRedisArgs`.
//!
//! This crate has zero dependency on `may` or any I/O runtime. It is pure data
//! and conversion logic.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph LR
//!     RV[RedisValue] --> RE[RedisError]
//!     RV --> FRV[FromRedisValue]
//!     RV --> TRA[ToRedisArgs]
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use base::RedisValue;
//!
//! let val = RedisValue::BulkString("hello".into());
//! assert!(matches!(val, RedisValue::BulkString(_)));
//! ```

pub mod error;
pub mod value;
