// codec — RESP protocol encoding/decoding
//
// Provides the wire-format layer that transforms RedisValue <-> bytes:
// - RESPWriter: encodes RedisValue into RESP2 wire format
// - RESPReader: decodes RESP2 wire format into RedisValue

//! # codec
//!
//! RESP protocol encoding/decoding: `RESPWriter`, `RESPReader`.
//!
//! This crate depends only on `bytes`, `itoa`, and `base`. No runtime dependency.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph LR
//!     RW[RESPWriter] --> RV[RedisValue]
//!     RR[RESPReader] --> RV
//!     bytes[bytes] --> RW
//!     bytes --> RR
//!     itoa[itoa] --> RW
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use may_redis::RedisValue;
//!
//! let mut writer = may_redis::codec::writer::RESPWriter::new();
//! writer.write_simple("OK");
//! let bytes = writer.take();
//! ```

pub mod reader;
pub mod writer;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
pub mod roundtrip;
