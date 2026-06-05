// Connection stream wrapper — wraps TcpStream or TlsStream.
//
// This enum provides a single type that the connection loop can use,
// avoiding the need to make spawn_connection_loop generic (which would
// require type erasure for JoinHandle).

use std::io;
use std::os::fd::AsRawFd;

use may::io::WaitIo;

/// Non-blocking read/write target for the connection loop.
///
/// Plain TCP uses the underlying `std::net::TcpStream` so `read`/`write`
/// return `WouldBlock` without yielding (see `may_postgres` `connection_loop`).
/// TLS uses [`TlsStream`] so rustls handles encryption.
pub(super) enum IoTarget<'a> {
    Sys(&'a mut std::net::TcpStream),
    #[cfg(feature = "tls")]
    Tls(&'a mut crate::tls::TlsStream),
}

pub enum ConnectionStream {
    /// Plain TCP stream.
    Tcp(may::net::TcpStream),
    #[cfg(feature = "tls")]
    /// TLS-wrapped stream (boxed to reduce enum size).
    Tls(Box<crate::tls::TlsStream>),
}

impl io::Read for ConnectionStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.read(buf),
        }
    }
}

impl io::Write for ConnectionStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.flush(),
        }
    }
}

impl ConnectionStream {
    /// Return the I/O object for non-blocking `nonblock_read` / `nonblock_write`.
    pub(super) fn io_target(&mut self) -> IoTarget<'_> {
        match self {
            Self::Tcp(stream) => IoTarget::Sys(stream.inner_mut()),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => IoTarget::Tls(stream),
        }
    }

    /// Socket fd and waker for connection setup (before the loop owns the stream).
    #[cfg_attr(not(feature = "tls"), allow(dead_code))]
    pub(super) fn socket_fd_and_waker(&mut self) -> (usize, may::io::WaitIoWaker) {
        match self {
            Self::Tcp(stream) => (stream.as_raw_fd() as usize, stream.waker()),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => {
                let tcp = stream.inner_mut();
                (tcp.as_raw_fd() as usize, tcp.waker())
            }
        }
    }
}

impl super::StreamHandle for ConnectionStream {
    fn wait_io(&mut self) -> i32 {
        match self {
            Self::Tcp(stream) => stream.wait_io(),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.wait_io(),
        }
    }
}
