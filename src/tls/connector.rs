// TLS handshake connector — builds rustls config and performs handshakes.
//
// Provides `TlsError`, `TlsConfig`, and `TlsConnector` with polling-based
// handshake using may coroutine yields.

use may::net::TcpStream;
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;
use std::time::Duration;

use super::config::TlsConfig as TlsConfigStruct;
use super::verifier::SkipVerifier;
use crate::tls::tls_stream::TlsStream;

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
// TlsConfig
// ---------------------------------------------------------------------------

/// TLS configuration for connecting to a Redis server.
#[derive(Clone)]
pub struct TlsConfig {
    /// Root CA certificates for server verification.
    pub root_certificates: super::config::RustlsRootCerts,
    /// Client certificate and private key for mTLS.
    pub client_certs: Option<super::config::ClientCerts>,
    /// Server hostname for SNI and certificate verification.
    pub server_name: String,
    /// Minimum TLS version (default: 1.2).
    pub min_version: super::config::TlsVersion,
    /// Maximum TLS version (default: 1.3).
    pub max_version: super::config::TlsVersion,
    /// Whether to verify the server certificate chain (default: true).
    pub verify_server: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            root_certificates: super::config::RustlsRootCerts::WebPkiRoots,
            client_certs: None,
            server_name: String::new(),
            min_version: super::config::TlsVersion::Tls12,
            max_version: super::config::TlsVersion::Tls13,
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
            (super::config::TlsVersion::Tls13, super::config::TlsVersion::Tls13) => {
                &[&rustls::version::TLS13][..]
            }
            (super::config::TlsVersion::Tls12, super::config::TlsVersion::Tls12) => {
                &[&rustls::version::TLS12][..]
            }
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
