// TCP — TCP socket management for the connection layer.
//
// Provides TcpConnector for establishing may-aware TCP connections
// to Redis servers with TCP_NODELAY and configurable timeouts.
// Provides SsrfConfig and SSRF guard functions.

#![allow(clippy::doc_markdown)]
#![allow(clippy::use_self)]
#![allow(clippy::single_match_else)]

use may::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

// ---------------------------------------------------------------------------
// ConnectionError
// ---------------------------------------------------------------------------

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
    /// SSRF protection: resolved address is in a deny-listed range.
    SsrfViolation(String),
    /// TLS handshake failed.
    #[cfg(feature = "tls")]
    Tls(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(msg) => write!(f, "resolve error: {msg}"),
            Self::Connect(msg) => write!(f, "connect error: {msg}"),
            Self::SetNodelay(msg) => write!(f, "set nodelay error: {msg}"),
            Self::Timeout(msg) => write!(f, "connection timeout: {msg}"),
            Self::SsrfViolation(msg) => write!(f, "SSRF violation: {msg}"),
            #[cfg(feature = "tls")]
            Self::Tls(msg) => write!(f, "TLS error: {msg}"),
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

// ---------------------------------------------------------------------------
// SSRF configuration
// ---------------------------------------------------------------------------

/// SSRF configuration for controlling which IP ranges are blocked.
#[derive(Debug, Clone, Copy)]
pub struct SsrfConfig {
    pub deny_private: bool,
    pub deny_link_local: bool,
    pub deny_loopback: bool,
}

impl Default for SsrfConfig {
    fn default() -> Self {
        Self {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: false,
        }
    }
}

/// Check if a resolved [`SocketAddr`] is allowed by the given [`SsrfConfig`].
#[must_use]
pub fn ssrf_allowed(addr: &SocketAddr, config: &SsrfConfig) -> bool {
    !config.is_blocked(addr)
}

impl SsrfConfig {
    fn is_blocked(self, addr: &SocketAddr) -> bool {
        match addr {
            SocketAddr::V4(v4) => self.is_blocked_v4(*v4.ip()),
            SocketAddr::V6(v6) => self.is_blocked_v6(v6.ip()),
        }
    }

    fn is_blocked_v4(self, addr: std::net::Ipv4Addr) -> bool {
        if !self.deny_private && !self.deny_link_local && !self.deny_loopback {
            return false;
        }
        let octets = addr.octets();
        let first = u32::from(octets[0]);
        let second = u32::from(octets[1]);

        if self.deny_private {
            if first == 10 {
                return true;
            }
            if first == 172 && (16..=31).contains(&second) {
                return true;
            }
            if first == 192 && second == 168 {
                return true;
            }
        }
        if self.deny_link_local && first == 169 && second == 254 {
            return true;
        }
        if self.deny_loopback && first == 127 {
            return true;
        }
        if first == 0 {
            return true;
        }
        if first == 100 && (64..=127).contains(&second) {
            return true;
        }
        if (224..=239).contains(&first) {
            return true;
        }
        if first >= 240 {
            return true;
        }
        false
    }

    const fn is_blocked_v6(self, addr: &std::net::Ipv6Addr) -> bool {
        if !self.deny_loopback && !self.deny_link_local && !self.deny_private {
            return addr.is_multicast() || addr.is_unspecified();
        }
        if self.deny_loopback && addr.is_loopback() {
            return true;
        }
        if self.deny_link_local && addr.is_unicast_link_local() {
            return true;
        }
        if self.deny_private && addr.is_unique_local() {
            return true;
        }
        addr.is_multicast() || addr.is_unspecified()
    }
}

// ---------------------------------------------------------------------------
// TcpConnector
// ---------------------------------------------------------------------------

/// TCP connector for establishing connections to Redis servers.
pub struct TcpConnector;

impl TcpConnector {
    /// Establish a TCP connection to the given host and port.
    /// Connect to a Redis server.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the TCP connection fails.
    pub fn connect(host: &str, port: u16) -> Result<TcpStream, ConnectionError> {
        Self::connect_with_timeout(host, port, Duration::from_secs(5))
    }

    /// Establish a TCP connection with SSRF protection enabled.
    /// Connect to a Redis server with SSRF protection.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if DNS resolution, TCP connect, or SSRF
    /// check fails.
    pub fn connect_with_ssrf_check(
        host: &str,
        port: u16,
        timeout: Duration,
        ssrf_config: SsrfConfig,
    ) -> Result<TcpStream, ConnectionError> {
        let addrs = resolve(host, port).map_err(ConnectionError::Resolve)?;

        for addr in &addrs {
            if ssrf_config.is_blocked(addr) {
                return Err(ConnectionError::SsrfViolation(format!(
                    "SSRF: resolved address {addr} is in a deny-listed range"
                )));
            }
        }

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

    /// Establish a TCP connection with a configurable timeout.
    /// Connect to a Redis server with a timeout.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the TCP connection fails or times out.
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
    /// Connect to a Redis server with a timeout specified in seconds.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the TCP connection fails or times out.
    pub fn connect_timeout(
        host: &str,
        port: u16,
        seconds: u32,
    ) -> Result<TcpStream, ConnectionError> {
        Self::connect_with_timeout(host, port, Duration::from_secs(u64::from(seconds)))
    }

    /// Parse a redis:// URL and connect with a 5-second default timeout.
    /// Connect to a Redis server given a URL.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the URL is invalid or connection fails.
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
    /// Connect to a Redis server given a URL with timeout.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the URL is invalid or connection fails.
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
pub(super) fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
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
