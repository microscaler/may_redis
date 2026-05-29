// may-redis — TLS support using rustls + ring
//
// Provides TlsConfig, TlsConnector, and TlsStream for encrypted Redis
// connections. The TLS layer wraps a raw may::net::TcpStream before the
// connection loop.

use may::net::TcpStream;
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, Der, PrivateKeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, RootCertStore};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// TlsVersion
// ---------------------------------------------------------------------------

/// TLS protocol version selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl TlsVersion {
    /// Convert to a rustls supported protocol version.
    #[must_use]
    pub fn to_supported(&self) -> &'static rustls::SupportedProtocolVersion {
        match self {
            Self::Tls12 => &rustls::version::TLS12,
            Self::Tls13 => &rustls::version::TLS13,
        }
    }

    /// Parse from a string ("1.2" or "1.3").
    ///
    /// # Errors
    /// Returns [`TlsError::InvalidTlsVersion`] if the string is not "1.2" or "1.3".
    pub fn from_str(s: &str) -> Result<Self, TlsError> {
        match s.trim() {
            "1.2" => Ok(Self::Tls12),
            "1.3" => Ok(Self::Tls13),
            _ => Err(TlsError::InvalidTlsVersion(format!(
                "unsupported TLS version: {s} (expected '1.2' or '1.3')"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// RustlsRootCerts
// ---------------------------------------------------------------------------

/// Root certificate source for server verification.
#[derive(Clone)]
pub enum RustlsRootCerts {
    /// Use Mozilla's root certificates from the webpki-roots crate.
    WebPkiRoots,
    /// Load from PEM-formatted certificate files on disk.
    Pem(Vec<PathBuf>),
    /// Load from in-memory DER-encoded certificates.
    Der(Vec<Vec<u8>>),
}

impl RustlsRootCerts {
    /// Convert to a rustls `RootCertStore`.
    ///
    /// # Errors
    /// Returns [`TlsError::Config`] if PEM files cannot be loaded.
    pub fn to_root_store(&self) -> Result<RootCertStore, TlsError> {
        let mut store = RootCertStore::empty();

        match self {
            Self::WebPkiRoots => {
                for ta in webpki_roots::TLS_SERVER_ROOTS {
                    store.roots.push(rustls::pki_types::TrustAnchor {
                        subject: Der::from_slice(ta.subject),
                        subject_public_key_info: Der::from_slice(ta.spki),
                        name_constraints: ta.name_constraints.map(|nc| Der::from_slice(nc)),
                    });
                }
            }
            Self::Pem(paths) => {
                for path in paths {
                    let file = std::fs::File::open(path).map_err(|e| {
                        TlsError::Config(format!(
                            "failed to open CA cert file {}: {e}",
                            path.display()
                        ))
                    })?;
                    let mut reader = io::BufReader::new(file);
                    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            TlsError::Config(format!(
                                "failed to parse PEM certs from {}: {e}",
                                path.display()
                            ))
                        })?;
                    store.add_parsable_certificates(certs);
                }
            }
            Self::Der(certs) => {
                for cert in certs {
                    store.add_parsable_certificates(vec![CertificateDer::from(cert.as_slice())]);
                }
            }
        }

        Ok(store)
    }
}

// ---------------------------------------------------------------------------
// ClientCerts (mTLS)
// ---------------------------------------------------------------------------

/// Client certificate and private key for mutual TLS.
#[derive(Clone)]
pub struct ClientCerts {
    /// DER-encoded client certificate chain (leaf first, then intermediates).
    pub certificates: Vec<Vec<u8>>,
    /// DER-encoded private key.
    pub private_key: Vec<u8>,
}

impl ClientCerts {
    /// Create from PEM-encoded certificate chain and private key.
    ///
    /// The `cert_pem` should contain the leaf certificate followed by any
    /// intermediate certificates. The `key_pem` should contain the private key
    /// in PKCS#8 or PKCS#1 format.
    ///
    /// # Errors
    /// Returns `TlsError::Config` if PEM parsing fails.
    pub fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, TlsError> {
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut &cert_pem[..])
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                TlsError::Config(format!("failed to parse client certificate PEM: {e}"))
            })?;

        let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut &key_pem[..])
            .map_err(|e| TlsError::Config(format!("failed to parse private key PEM: {e}")))?
            .ok_or_else(|| TlsError::Config("no private key found in PEM data".to_string()))?;

        Ok(Self {
            certificates: certs.iter().map(|c| c.to_vec()).collect(),
            private_key: key.secret_der().to_vec(),
        })
    }

    /// Create from DER-encoded certificate chain and private key directly.
    #[must_use]
    pub fn from_der(certificates: Vec<Vec<u8>>, private_key: Vec<u8>) -> Self {
        Self {
            certificates,
            private_key,
        }
    }
}

// ---------------------------------------------------------------------------
// TlsError
// ---------------------------------------------------------------------------

/// Error type for TLS operations.
#[derive(Debug)]
pub enum TlsError {
    /// Failed to build rustls configuration.
    Config(String),
    /// TLS handshake timed out.
    HandshakeTimeout,
    /// TLS handshake failed with a rustls error.
    Handshake(String),
    /// Certificate verification failed.
    Verification(String),
    /// Invalid TLS version string.
    InvalidTlsVersion(String),
    /// Server requested a client certificate but none was provided.
    ClientCertRequired(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "TLS config error: {msg}"),
            Self::HandshakeTimeout => write!(f, "TLS handshake timed out"),
            Self::Handshake(msg) => write!(f, "TLS handshake error: {msg}"),
            Self::Verification(msg) => write!(f, "Certificate verification failed: {msg}"),
            Self::InvalidTlsVersion(msg) => write!(f, "Invalid TLS version: {msg}"),
            Self::ClientCertRequired(msg) => write!(f, "Client certificate required: {msg}"),
        }
    }
}

impl std::error::Error for TlsError {}

// ---------------------------------------------------------------------------
// SkipVerifier — certificate verifier that accepts any certificate
// ---------------------------------------------------------------------------

/// A certificate verifier that skips verification (for debugging only).
///
/// # Security
///
/// This verifier accepts **any** server certificate without validation.
/// NEVER use this in production.
#[derive(Debug)]
struct SkipVerifier {
    inner: Arc<WebPkiServerVerifier>,
}

impl rustls::client::danger::ServerCertVerifier for SkipVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

// ---------------------------------------------------------------------------
// TlsConfig
// ---------------------------------------------------------------------------

/// TLS configuration for connecting to a Redis server.
#[derive(Clone)]
pub struct TlsConfig {
    /// Root CA certificates for server verification.
    pub root_certificates: RustlsRootCerts,
    /// Client certificate and private key for mTLS.
    pub client_certs: Option<ClientCerts>,
    /// Server hostname for SNI and certificate verification.
    pub server_name: String,
    /// Minimum TLS version (default: 1.2).
    pub min_version: TlsVersion,
    /// Maximum TLS version (default: 1.3).
    pub max_version: TlsVersion,
    /// Whether to verify the server certificate chain (default: true).
    pub verify_server: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            client_certs: None,
            server_name: String::new(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: true,
        }
    }
}

impl TlsConfig {
    /// Build a rustls `ClientConfig` from this configuration.
    ///
    /// # Errors
    /// Returns [`TlsError`] if the configuration is invalid.
    pub fn into_config(self) -> Result<ClientConfig, TlsError> {
        // Validate version bounds.
        if self.min_version > self.max_version {
            return Err(TlsError::Config(format!(
                "min_version {:?} is greater than max_version {:?}",
                self.min_version, self.max_version,
            )));
        }

        // Install the ring crypto provider if not already installed.
        let provider = default_provider();
        provider.clone().install_default().map_err(|_| {
            TlsError::Config("failed to install crypto provider (already installed?)".to_string())
        })?;

        // Determine which protocol versions to allow.
        let versions = match (self.min_version, self.max_version) {
            (TlsVersion::Tls13, TlsVersion::Tls13) => &[&rustls::version::TLS13][..],
            (TlsVersion::Tls12, TlsVersion::Tls12) => &[&rustls::version::TLS12][..],
            _ => rustls::DEFAULT_VERSIONS,
        };

        let builder = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(versions)
            .map_err(|e| TlsError::Config(format!("protocol version error: {e}")))?;

        // Build root certificates.
        let root_store = self.root_certificates.to_root_store()?;

        // Build the config: either with standard verification or with SkipVerifier.
        let builder = if self.verify_server {
            builder.with_root_certificates(root_store)
        } else {
            let verifier = WebPkiServerVerifier::builder(Arc::new(root_store))
                .build()
                .map_err(|e| TlsError::Config(format!("failed to build verifier: {e}")))?;
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(SkipVerifier { inner: verifier }))
        };

        // Apply client certs for mTLS.
        // `with_client_auth_cert` returns `Result<ClientConfig, Error>`,
        // while `with_no_client_auth` returns `ClientConfig` directly.
        let config = if let Some(client_certs) = self.client_certs {
            builder
                .with_client_auth_cert(
                    client_certs
                        .certificates
                        .into_iter()
                        .map(CertificateDer::from)
                        .collect(),
                    PrivateKeyDer::try_from(client_certs.private_key)
                        .map_err(|e| TlsError::Config(format!("invalid private key: {e}")))?,
                )
                .map_err(|e| TlsError::Config(format!("failed to set client certs: {e}")))?
        } else {
            builder.with_no_client_auth()
        };

        Ok(config)
    }
}

// ---------------------------------------------------------------------------
// TlsStream
// ---------------------------------------------------------------------------

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
    conn: rustls::ClientConnection,
    stream: may::net::TcpStream,
}

impl TlsStream {
    pub fn new(conn: rustls::ClientConnection, stream: may::net::TcpStream) -> Self {
        Self { conn, stream }
    }

    /// Return a mutable reference to the underlying `TcpStream`.
    ///
    /// Used by the connection loop for `wait_io()` (epoll registration)
    /// and for feeding raw socket reads/writes into the rustls state machine.
    pub fn inner_mut(&mut self) -> &mut may::net::TcpStream {
        &mut self.stream
    }

    /// Return the raw inner tcp stream.
    #[must_use]
    pub fn inner(&self) -> &may::net::TcpStream {
        &self.stream
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.conn.reader().read(buf)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.conn.writer().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.conn.writer().flush()
    }
}

// ---------------------------------------------------------------------------
// TlsConnector
// ---------------------------------------------------------------------------

/// Handles TLS handshakes using a polling pattern with may coroutine yields.
pub struct TlsConnector;

impl TlsConnector {
    /// Perform a TLS handshake on a raw TCP stream.
    ///
    /// The handshake uses a polling loop with `may::coroutine::yield_now()`
    /// instead of async-await. Each iteration calls `complete_io` which
    /// performs the necessary read/write cycles to advance the handshake.
    ///
    /// # Arguments
    /// * `stream` — The raw TCP stream (must be non-blocking)
    /// * `config` — TLS configuration (root certs, client certs, SNI, etc.)
    /// * `timeout` — Maximum duration to wait for the handshake to complete
    ///
    /// # Errors
    /// Returns [`TlsError`] if the handshake fails or times out.
    pub fn handshake(
        stream: TcpStream,
        config: &TlsConfig,
        timeout: Duration,
    ) -> Result<TlsStream, TlsError> {
        // Clone the server name first so we don't borrow config while
        // into_config() consumes self.
        let server_name_raw = if config.server_name.is_empty() {
            "localhost".to_string()
        } else {
            config.server_name.clone()
        };

        // Convert to ServerName<'static> BEFORE calling into_config().
        let server_name = ServerName::try_from(server_name_raw)
            .map_err(|e| TlsError::Config(format!("invalid server name for SNI: {e}")))?;

        let tls_config = config.clone().into_config()?;

        let conn = rustls::ClientConnection::new(Arc::new(tls_config), server_name)
            .map_err(|e| TlsError::Config(format!("failed to create TLS connection: {e}")))?;

        let mut tls_stream = TlsStream::new(conn, stream);

        // Polling handshake loop.
        // complete_io returns (usize, usize) — bytes read, bytes written.
        // We loop until the connection says handshake is done.
        let deadline = std::time::Instant::now() + timeout;
        let mut idle = 0u32;

        loop {
            // Because conn and stream are separate fields we can borrow them
            // independently — no double-mutable-borrow on tls_stream.
            let (read_done, write_done) = tls_stream
                .conn
                .complete_io(&mut tls_stream.stream)
                .map_err(|e| TlsError::Handshake(format!("I/O error during handshake: {e}")))?;

            // If handshake is done, we're done.
            if !tls_stream.conn.is_handshaking() {
                break;
            }

            // If we didn't do anything this iteration, yield to avoid busy-waiting.
            if read_done == 0 && write_done == 0 {
                if std::time::Instant::now() >= deadline {
                    return Err(TlsError::HandshakeTimeout);
                }
                idle += 1;
                if idle > 100 {
                    // Safety valve: if we've yielded 100 times without progress, abort.
                    return Err(TlsError::HandshakeTimeout);
                }
                may::coroutine::yield_now();
            } else {
                idle = 0; // reset on progress
            }
        }

        Ok(tls_stream)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_version_from_str_12() {
        assert_eq!(TlsVersion::from_str("1.2").unwrap(), TlsVersion::Tls12);
    }

    #[test]
    fn test_tls_version_from_str_13() {
        assert_eq!(TlsVersion::from_str("1.3").unwrap(), TlsVersion::Tls13);
    }

    #[test]
    fn test_tls_version_from_str_invalid() {
        let err = TlsVersion::from_str("1.1").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_from_str_empty() {
        let err = TlsVersion::from_str("").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_config_defaults() {
        let config = TlsConfig::default();
        assert!(matches!(
            config.root_certificates,
            RustlsRootCerts::WebPkiRoots
        ));
        assert!(config.client_certs.is_none());
        assert_eq!(config.min_version, TlsVersion::Tls12);
        assert_eq!(config.max_version, TlsVersion::Tls13);
        assert!(config.verify_server);
    }

    #[test]
    fn test_tls_version_ordering() {
        assert!(TlsVersion::Tls12 < TlsVersion::Tls13);
    }

    #[test]
    fn test_tls_config_min_gt_max() {
        let config = TlsConfig {
            min_version: TlsVersion::Tls13,
            max_version: TlsVersion::Tls12,
            ..TlsConfig::default()
        };
        let result = config.into_config();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::Config(_)));
    }
}
