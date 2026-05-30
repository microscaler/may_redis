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

/// Trait for connection loop stream handles.
///
/// Abstracts over [`may::net::TcpStream`] and [`tls::TlsStream`] so the
/// epoll loop can read/write via `io::Read`/`io::Write` and wait on
/// epoll via `wait_io()`.
pub(crate) trait StreamHandle: std::io::Read + std::io::Write {
    /// Return the inner socket for epoll registration.
    ///
    /// For plain TCP this returns `&mut TcpStream`.
    /// For TLS this returns `&mut TcpStream` (the underlying socket).
    fn inner_mut(&mut self) -> &mut may::net::TcpStream;

    /// Wait for I/O readiness via epoll.
    fn wait_io(&mut self) -> i32;
}

/// Blanket impl for anything that already implements `may::io::WaitIo`
/// and has a `may::net::TcpStream`-compatible `inner_mut()`.
impl StreamHandle for may::net::TcpStream {
    fn inner_mut(&mut self) -> &mut may::net::TcpStream {
        self
    }

    fn wait_io(&mut self) -> i32 {
        #[allow(clippy::cast_possible_wrap)]
        {
            may::io::WaitIo::wait_io(self) as i32
        }
    }
}

#[allow(clippy::module_inception)]
pub mod connection;
pub mod connection_limits;
mod connection_stream;
#[cfg(test)]
mod connection_tests;
#[cfg(feature = "tls")]
mod connection_tls;
pub mod dispatch;
pub mod epoll_loop;
pub mod io_read;
pub mod io_write;
pub mod tcp;
#[cfg(test)]
mod tcp_tests;
#[cfg(test)]
mod test_limits;

pub use connection::{Connection, Request};
pub use connection_limits::ConnectionLimitError;
pub use tcp::{ssrf_allowed, ConnectionError, SsrfConfig, TcpConnector};
