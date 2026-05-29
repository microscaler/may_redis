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
//! This crate depends on `bytes`, `log`, `may`, `base`, and `codec`.
//!
//! ## Architecture
//!
//! ```text
//! +---------------------+     +-------------------------+
//! |  Application Co     |     |  Connection Co (go!)    |
//! |                     |     |                         |
//! |  Connection::send() | --> |  mpsc Queue<Request>    |
//! |                     |     |  + epoll loop            |
//! |  spsc::Receiver     | <-- |  + read_buf / write_buf  |
//! |  from spsc channel  |     |  + RESPReader decode     |
//! +---------------------+     +-------------------------+
//!
//! ## Usage
//!
//! ```ignore
//! use may_redis::connection::{Connection, Request, SsrfConfig, ssrf_allowed};
//! use may::sync::spsc;
//!
//! // SSRF-safe connection
//! let ssrf = SsrfConfig {
//!     deny_private: true,
//!     deny_link_local: true,
//!     deny_loopback: true,
//! };
//! may::run(|| {
//!     may::go(|| {
//!         let conn = Connection::connect_with_ssrf_protection(
//!             "localhost", 6379,
//!             std::time::Duration::from_secs(5),
//!             ssrf,
//!         ).unwrap();
//!         let (tx, rx) = spsc::channel();
//!         let request = Request::new(
//!             vec![b'*1\r\n$4\r\nPING\r\n'],
//!             tx,
//!         );
//!         conn.send(request);
//!         let response: may_redis::RedisValue = rx.recv().unwrap();
//!         println!("{response:?}");
//!     }).join();
//! });
//! ```

#[allow(clippy::module_inception)]
pub mod connection;
pub mod tcp;

pub use connection::{Connection, ConnectionLimitError, Request};
pub use tcp::{ssrf_allowed, ConnectionError, SsrfConfig, TcpConnector};
