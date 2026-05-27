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
//! use crate::connection::{Connection, Request};
//! use may::sync::spsc;
//!
//! may::run(|| {
//!     may::go(|| {
//!         let conn = Connection::connect("127.0.0.1", 6379).unwrap();
//!         let (tx, rx) = spsc::channel();
//!         let request = Request::new(
//!             vec![b'*1\r\n$4\r\nPING\r\n'],
//!             tx,
//!         );
//!         conn.send(request);
//!         let response: RedisValue = rx.recv().unwrap();
//!         println!("{response:?}");
//!     }).join();
//! });
//! ```

pub mod connection;
pub mod tcp;

pub use connection::{Connection, Request};
pub use tcp::{ConnectionError, TcpConnector};
