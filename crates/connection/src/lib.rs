// connection — Redis connection loop
//
// Provides the network I/O layer:
// - Epoll-based single-coroutine connection loop
// - TCP socket management with non-blocking I/O
// - Request-response matching via monotonically increasing tags
// - Spsc channel per-request for response dispatch

//! # connection
//!
//! Redis connection loop: epoll, TCP, coroutine management.
//!
//! This crate depends on `bytes`, `log`, `may`, `socket2`, `base`, and `codec`.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph TB
//!     subgraph "Connection Loop"
//!         CL[go! epoll loop]
//!         TCP[TCP Stream]
//!         RW[Read/Write buffers]
//!     end
//!     subgraph "Request Pipeline"
//!         Q[mpsc Queue Request]
//!         Tag[Monotonic tag]
//!         S[spsc Receiver Response]
//!     end
//!     CL --> TCP
//!     TCP --> RW
//!     Q --> CL
//!     CL --> Tag
//!     Tag --> S
//!     S --> CL
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use connection::Connection;
//!
//! let conn = Connection::connect("127.0.0.1:6379").await;
//! ```

pub mod epoll;
pub mod tcp;
