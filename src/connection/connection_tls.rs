// TLS connection constructors for `Connection`.
//
// This module was extracted from `connection.rs` to keep that file under the
// 350-line limit. It owns all TLS-related construction logic.

use std::os::fd::AsRawFd;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use may::io::WaitIo;
use may::queue::mpsc::Queue;

use super::connection_limits::{DEFAULT_MAX_QUEUE_DEPTH, DEFAULT_MAX_REQUEST_SIZE};
use super::connection_stream::ConnectionStream;
use super::epoll_loop::spawn_connection_loop;
use super::tcp::{self, ConnectionError, TcpConnector};
use super::StreamHandle;
use crate::tls::TlsConnector;

// ---------------------------------------------------------------------------
// TLS connection constructors
// ---------------------------------------------------------------------------

/// Establish a TCP connection and perform TLS handshake.
///
/// # Arguments
/// * `host` - Server hostname or IP address
/// * `port` - Server port (typically 6380 for TLS)
/// * `tls_config` - TLS configuration
/// * `timeout_secs` - Connection timeout in seconds
///
/// # Errors
/// Returns [`ConnectionError::Tls`] if the TCP connection or TLS handshake fails.
#[cfg(feature = "tls")]
pub(super) fn connect_tls(
    host: &str,
    port: u16,
    tls_config: &super::super::tls::TlsConfig,
    timeout_secs: u32,
) -> Result<super::connection::Connection, ConnectionError> {
    // 1. TCP connect
    let stream = TcpConnector::connect_timeout(host, port, timeout_secs)?;

    // 2. TLS handshake
    let tls_stream = TlsConnector::handshake(
        stream,
        tls_config,
        std::time::Duration::from_secs(u64::from(timeout_secs)),
    )
    .map_err(|e| ConnectionError::Tls(format!("TLS handshake failed: {e}")))?;

    // 3. Create connection loop with TLS stream
    Ok(from_tls_stream(ConnectionStream::Tls(Box::new(tls_stream))))
}

/// Establish a TCP connection with SSRF protection and perform TLS handshake.
///
/// # Arguments
/// * `host` - Server hostname or IP address
/// * `port` - Server port (typically 6380 for TLS)
/// * `tls_config` - TLS configuration
/// * `timeout_secs` - Connection timeout in seconds
/// * `ssrf_config` - Configuration for which IP ranges to block
///
/// # Errors
/// Returns [`ConnectionError::Tls`] if the TCP connection, SSRF check, or TLS
/// handshake fails.
#[cfg(feature = "tls")]
pub(super) fn connect_tls_with_ssrf(
    host: &str,
    port: u16,
    tls_config: &super::super::tls::TlsConfig,
    timeout_secs: u32,
    ssrf_config: tcp::SsrfConfig,
) -> Result<super::connection::Connection, ConnectionError> {
    // 1. TCP connect with SSRF protection
    let stream = TcpConnector::connect_with_ssrf_check(
        host,
        port,
        std::time::Duration::from_secs(u64::from(timeout_secs)),
        ssrf_config,
    )?;

    // 2. TLS handshake
    let tls_stream = TlsConnector::handshake(
        stream,
        tls_config,
        std::time::Duration::from_secs(u64::from(timeout_secs)),
    )
    .map_err(|e| ConnectionError::Tls(format!("TLS handshake failed: {e}")))?;

    // 3. Create connection loop with TLS stream
    Ok(from_tls_stream(ConnectionStream::Tls(Box::new(tls_stream))))
}

// ---------------------------------------------------------------------------
// Internal TLS helpers
// ---------------------------------------------------------------------------

/// Create a Connection from an already-handshaked TLS stream.
///
/// The TLS handshake MUST already be complete before calling this.
/// The epoll loop will wrap the TLS stream the same way it wraps TCP.
#[cfg(feature = "tls")]
fn from_tls_stream(mut tls_stream: ConnectionStream) -> super::connection::Connection {
    let id = tls_stream.inner_mut().as_raw_fd() as usize;
    let waker = tls_stream.inner_mut().waker();
    let req_queue = Arc::new(Queue::new());
    let pending_count = Arc::new(AtomicUsize::new(0));
    let io_handle = spawn_connection_loop(tls_stream, req_queue.clone(), pending_count.clone());

    super::connection::Connection {
        io_handle,
        req_queue,
        waker,
        id,
        tag_counter: Arc::new(AtomicUsize::new(0)),
        max_queue_depth: DEFAULT_MAX_QUEUE_DEPTH,
        max_request_size: DEFAULT_MAX_REQUEST_SIZE,
        pending_count,
        ssrf_config: None,
    }
}
