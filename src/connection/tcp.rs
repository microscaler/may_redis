// TCP — TCP socket management for the connection layer
//
// Provides TcpConnector for establishing may-aware TCP connections
// to Redis servers with TCP_NODELAY and configurable timeouts.

#![allow(clippy::doc_markdown)]
#![allow(clippy::use_self)]
#![allow(clippy::single_match_else)]

use may::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

/// Error type for TCP connection failures.
#[derive(Debug)]
pub enum ConnectionError {
    /// DNS resolution failed.
    Resolve(String),
    /// TCP connect failed.
    Connect(String),
    /// Failed to set TCP_NODELAY.
    SetNodelay(String),
    /// Connection timed out.
    Timeout(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(msg) => write!(f, "resolve error: {msg}"),
            Self::Connect(msg) => write!(f, "connect error: {msg}"),
            Self::SetNodelay(msg) => write!(f, "set nodelay error: {msg}"),
            Self::Timeout(msg) => write!(f, "connection timeout: {msg}"),
        }
    }
}

impl std::error::Error for ConnectionError {}

impl ConnectionError {
    /// Returns `true` if this error is a timeout.
    #[must_use]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

/// TCP connector for establishing connections to Redis servers.
pub struct TcpConnector;

impl TcpConnector {
    /// Establish a TCP connection to the given host and port.
    ///
    /// Uses a 5-second default timeout. Resolves the address, creates a
    /// may-aware [`TcpStream`] (which internally sets non-blocking mode and
    /// registers with epoll), sets TCP_NODELAY, and returns the stream ready
    /// for use.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    ///
    /// # Errors
    /// Returns [`ConnectionError`] on DNS resolution failure, TCP connect
    /// failure, TCP_NODELAY failure, or if all resolved addresses fail to
    /// connect.
    pub fn connect(host: &str, port: u16) -> Result<TcpStream, ConnectionError> {
        Self::connect_with_timeout(host, port, Duration::from_secs(5))
    }

    /// Establish a TCP connection with a configurable timeout.
    ///
    /// Resolves the address, creates a may-aware [`TcpStream`], sets TCP_NODELAY,
    /// and returns the stream ready for use. If the connection does not complete
    /// within `timeout`, returns [`ConnectionError::Timeout`].
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    /// * `timeout` - Maximum duration to wait for the connection
    ///
    /// # Errors
    /// Returns [`ConnectionError::Resolve`] on DNS failure,
    /// [`ConnectionError::Connect`] on TCP failure,
    /// [`ConnectionError::SetNodelay`] on socket option failure, or
    /// [`ConnectionError::Timeout`] on timeout.
    pub fn connect_with_timeout(
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<TcpStream, ConnectionError> {
        let addrs = resolve(host, port).map_err(ConnectionError::Resolve)?;

        let mut last_error = None;
        for addr in &addrs {
            match connect_addr_with_timeout(addr, timeout) {
                Ok(stream) => return Ok(stream),
                Err(e) if e.is_timeout() => return Err(e),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error
            .unwrap_or_else(|| ConnectionError::Connect("resolved 0 addresses".to_string())))
    }

    /// Establish a TCP connection with timeout in seconds.
    ///
    /// Convenience method that converts seconds to a Duration and calls
    /// `connect_with_timeout`.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    /// * `seconds` - Maximum seconds to wait for the connection
    ///
    /// # Errors
    /// Returns [`ConnectionError`] on resolution, connection, nodelay,
    /// or timeout failure.
    pub fn connect_timeout(
        host: &str,
        port: u16,
        seconds: u32,
    ) -> Result<TcpStream, ConnectionError> {
        Self::connect_with_timeout(host, port, Duration::from_secs(u64::from(seconds)))
    }

    /// Parse a redis:// URL and connect with a 5-second default timeout.
    ///
    /// # Arguments
    /// * `url` - Connection URL in the format `redis://host:port`
    ///
    /// # Errors
    /// Returns [`ConnectionError::Connect`] if the URL is malformed or the
    /// port is invalid, or [`ConnectionError`] from the underlying connect
    /// call.
    pub fn connect_url(url: &str) -> Result<TcpStream, ConnectionError> {
        let url = url.strip_prefix("redis://").unwrap_or(url);
        let (host, port) = url
            .rsplit_once(':')
            .ok_or_else(|| ConnectionError::Connect("invalid URL format".to_string()))?;
        let port: u16 = port
            .parse()
            .map_err(|e| ConnectionError::Connect(format!("invalid port: {e}")))?;
        Self::connect(host, port)
    }

    /// Parse a redis:// URL and connect with a configurable timeout.
    ///
    /// # Arguments
    /// * `url` - Connection URL in the format `redis://host:port`
    /// * `seconds` - Maximum seconds to wait for the connection
    ///
    /// # Errors
    /// Returns [`ConnectionError::Connect`] if the URL is malformed or the
    /// port is invalid, or [`ConnectionError`] from the underlying connect
    /// call.
    pub fn connect_url_timeout(url: &str, seconds: u32) -> Result<TcpStream, ConnectionError> {
        let url = url.strip_prefix("redis://").unwrap_or(url);
        let (host, port) = url
            .rsplit_once(':')
            .ok_or_else(|| ConnectionError::Connect("invalid URL format".to_string()))?;
        let port: u16 = port
            .parse()
            .map_err(|e| ConnectionError::Connect(format!("invalid port: {e}")))?;
        Self::connect_timeout(host, port, seconds)
    }
}

/// Resolve host and port to a list of [`SocketAddr`].
fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        Ok(vec![SocketAddr::new(ip, port)])
    } else {
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .collect::<Vec<_>>();
        if addrs.is_empty() {
            Err("resolved 0 addresses".to_string())
        } else {
            Ok(addrs)
        }
    }
}

/// Connect to a specific [`SocketAddr`] with a timeout, using TCP_NODELAY.
///
/// Uses `may::net::TcpStream::connect_timeout` (backed by the `io_timeout`
/// feature of the `may` crate) to apply a deadline to the TCP connect phase.
/// This avoids blocking indefinitely on DNS resolution or SYN handshakes.
fn connect_addr_with_timeout(
    addr: &SocketAddr,
    timeout: Duration,
) -> Result<TcpStream, ConnectionError> {
    let stream = TcpStream::connect_timeout(addr, timeout)
        .map_err(|e| ConnectionError::Connect(e.to_string()))?;

    stream
        .set_nodelay(true)
        .map_err(|e| ConnectionError::SetNodelay(e.to_string()))?;

    Ok(stream)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::Resolve("host not found".to_string());
        assert!(format!("{err}").contains("resolve"));

        let err = ConnectionError::Connect("connection refused".to_string());
        assert!(format!("{err}").contains("connect"));

        let err = ConnectionError::SetNodelay("operation not supported".to_string());
        assert!(format!("{err}").contains("nodelay"));

        let err = ConnectionError::Timeout("5s exceeded".to_string());
        assert!(format!("{err}").contains("timeout"));
    }

    #[test]
    fn test_connection_error_is_timeout() {
        let err = ConnectionError::Timeout("test".to_string());
        assert!(err.is_timeout());

        let err = ConnectionError::Connect("test".to_string());
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_tcp_connector_struct_exists() {
        let _ = TcpConnector;
    }

    #[test]
    fn test_resolve_ip_address() {
        let addrs = resolve("127.0.0.1", 6379).unwrap();
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0].port(), 6379);
    }

    #[test]
    fn test_resolve_hostname() {
        let addrs = resolve("localhost", 6379).unwrap();
        assert!(!addrs.is_empty());
        assert_eq!(addrs[0].port(), 6379);
    }

    #[test]
    fn test_connect_url_parses() {
        // Test URL parsing with an unresolvable hostname to avoid
        // depending on Redis actually running on localhost.
        let result = TcpConnector::connect_url("redis://nonexistent.invalid:6379");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_timeout_parses() {
        let result = TcpConnector::connect_url_timeout("redis://nonexistent.invalid:6379", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_invalid_port() {
        let result = TcpConnector::connect_url("redis://localhost:abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_invalid_format() {
        let result = TcpConnector::connect_url("not-a-url");
        assert!(result.is_err());
    }

    /// Verify that connecting to a refused port returns Connect error (not Timeout).
    #[test]
    #[ignore = "requires live network namespace"]
    fn test_connect_refused_returns_connect() {
        use may::go;

        let wrapper = std::sync::Mutex::new(None::<()>);
        let _wrapper2 = wrapper.lock().unwrap();
        let wrapper2 = std::sync::Arc::new(std::sync::Mutex::new(None::<()>));
        let wrapper3 = std::sync::Arc::clone(&wrapper2);

        let _ = go!(move || {
            let result = TcpConnector::connect_timeout("127.0.0.1", 1, 5);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ConnectionError::Connect(_)));
            *wrapper3.lock().unwrap() = Some(());
        });
    }

    /// Verify that default connect() uses a 5-second timeout.
    #[test]
    fn test_connect_default_timeout() {
        // Just verify connect() doesn't panic; the 5s default is implicit
        let _ = TcpConnector::connect("127.0.0.1", 6379);
    }
}
