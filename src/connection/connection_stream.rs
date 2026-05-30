// Connection stream wrapper — wraps TcpStream or TlsStream.
//
// This enum provides a single type that the connection loop can use,
// avoiding the need to make spawn_connection_loop generic (which would
// require type erasure for JoinHandle).

use std::io;

pub enum ConnectionStream {
    /// Plain TCP stream.
    Tcp(may::net::TcpStream),
    #[cfg(feature = "tls")]
    /// TLS-wrapped stream (boxed to reduce enum size).
    Tls(Box<crate::tls::TlsStream>),
}

#[allow(dead_code)]
impl ConnectionStream {
    #[must_use]
    pub(crate) fn into_tcp(self) -> Option<may::net::TcpStream> {
        match self {
            Self::Tcp(stream) => Some(stream),
            #[cfg(feature = "tls")]
            Self::Tls(_) => None,
        }
    }

    #[cfg(feature = "tls")]
    #[must_use]
    pub(crate) fn into_tls(self) -> Option<crate::tls::TlsStream> {
        match self {
            Self::Tcp(_) => None,
            Self::Tls(stream) => Some(*stream),
        }
    }
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

impl super::StreamHandle for ConnectionStream {
    fn inner_mut(&mut self) -> &mut may::net::TcpStream {
        match self {
            Self::Tcp(stream) => stream,
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.inner_mut(),
        }
    }

    fn wait_io(&mut self) -> i32 {
        match self {
            Self::Tcp(stream) => stream.wait_io(),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.wait_io(),
        }
    }
}
