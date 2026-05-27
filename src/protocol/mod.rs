// protocol — Redis command protocol
//
// Provides the command construction layer:
// - CommandBuilder: builds RESP commands from Rust types
// - Commands: trait mirroring the redis crate API surface

//! # protocol
//!
//! Redis command protocol: `CommandBuilder`, `Commands` trait.
//!
//! This crate depends on `bytes`, `log`, `may`, `base`, and `codec`.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph LR
//!     CB[CommandBuilder] --> CT[Commands trait]
//!     CB --> RV[RedisValue]
//!     CB --> RW[RESPWriter from codec]
//!     may[may runtime] --> CB
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use crate::protocol::Commands;
//!
//! let cmd = Commands::get("mykey");
//! ```

pub mod builder;
pub mod commands;
