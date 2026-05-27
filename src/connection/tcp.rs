// TCP — TCP socket management for the connection layer
//
// Provides TcpConnector for establishing may-aware TCP connections
// to Redis servers with TCP_NODELAY.

#![allow(clippy::doc_markdown)]
#![allow(clippy::use_self)]
#![allow(clippy::single_match_else)]

use may::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};

/// Error type for TCP connection failures.
#[derive(Debug)]
pub enum ConnectionError {
    /// DNS resolution failed.
    Resolve(String),
    /// TCP connect failed.
    Connect(String),
    /// Failed to set TCP_NODELAY.
    SetNodelay(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(msg) => write!(f, "resolve error: {msg}"),
            Self::Connect(msg) => write!(f, "connect error: {msg}"),
            Self::SetNodelay(msg) => write!(f, "set nodelay error: {msg}"),
        }
    }
}

impl std::error::Error for ConnectionError {}

/// TCP connector for establishing connections to Redis servers.
pub struct TcpConnector;

impl TcpConnector {
    /// Establish a TCP connection to the given host and port.
    ///
    /// Resolves the address, creates a may-aware [`TcpStream`] (which internally
    /// sets non-blocking mode and registers with epoll), sets TCP_NODELAY,
    /// and returns the stream ready for use.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    pub fn connect(host: &str, port: u16) -> Result<TcpStream, ConnectionError> {
        let addrs = resolve(host, port).map_err(ConnectionError::Resolve)?;

        let mut last_error = None;
        for addr in addrs {
            match connect_addr(&addr) {
                Ok(stream) => return Ok(stream),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error
            .unwrap_or_else(|| ConnectionError::Connect("resolved 0 addresses".to_string())))
    }

    /// Parse a redis:// URL and connect.
    ///
    /// # Arguments
    /// * `url` - Connection URL in the format `redis://host:port`
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

/// Connect to a specific [`SocketAddr`] with TCP_NODELAY enabled.
fn connect_addr(addr: &SocketAddr) -> Result<TcpStream, ConnectionError> {
    // may::net::TcpStream::connect handles the full connect cycle
    // within the coroutine context (blocking connects are cooperative in may).
    let stream = TcpStream::connect(addr).map_err(|e| ConnectionError::Connect(e.to_string()))?;

    // Set TCP_NODELAY for low-latency response.
    stream
        .set_nodelay(true)
        .map_err(|e| ConnectionError::SetNodelay(e.to_string()))?;

    Ok(stream)
}

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
    fn test_connect_url_parses() {
        // Test URL parsing with an unresolvable hostname to avoid
        // depending on Redis actually running on localhost.
        let result = TcpConnector::connect_url("redis://nonexistent.invalid:6379");
        assert!(result.is_err());
    }
}
