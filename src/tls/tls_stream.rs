// TLS stream wrapper for may-redis.
//
// Wraps a rustls `ClientConnection` and the underlying TCP socket,
// implementing `Read` and `Write` for integration with the connection
// loop's non-blocking I/O helpers.

use may::net::TcpStream;
use std::io;

/// Wraps a rustls `ClientConnection` and the underlying TCP socket.
///
/// Holds the `ClientConnection` and `TcpStream` as **separate** fields so
/// that `ClientConnection::complete_io` can borrow them independently —
/// avoiding the double-mutable-borrow that `StreamOwned` would force.
///
/// Implements `Read` / `Write` via the rustls `Reader` / `Writer` helpers,
/// integrating with the existing `nonblock_read` / `nonblock_write` helpers
/// in the connection layer.
pub struct TlsStream {
    pub(crate) conn: rustls::ClientConnection,
    pub(crate) stream: TcpStream,
}

impl TlsStream {
    pub const fn new(conn: rustls::ClientConnection, stream: TcpStream) -> Self {
        Self { conn, stream }
    }

    /// Return a mutable reference to the underlying `TcpStream`.
    ///
    /// Used by the connection loop for `wait_io()` (epoll registration)
    /// and for feeding raw socket reads/writes into the rustls state machine.
    pub const fn inner_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Return the raw inner tcp stream.
    #[must_use]
    pub const fn inner(&self) -> &TcpStream {
        &self.stream
    }
}

impl io::Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.conn.reader().read(buf)
    }
}

impl io::Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.conn.writer().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.conn.writer().flush()
    }
}

// Re-export for connection module
use crate::connection::StreamHandle;

impl StreamHandle for TlsStream {
    fn inner_mut(&mut self) -> &mut may::net::TcpStream {
        &mut self.stream
    }

    fn wait_io(&mut self) -> i32 {
        self.stream.wait_io()
    }
}
