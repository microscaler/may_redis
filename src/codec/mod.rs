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
//! ```ignore
//! use crate::codec::RESPWriter;
//! use crate::base::RedisValue;
//!
//! let mut buf = Vec::new();
//! RESPWriter::encode(&mut buf, &RedisValue::SimpleString("OK".into())).unwrap();
//! ```

pub mod reader;
pub mod writer;

#[cfg(test)]
pub mod roundtrip;
