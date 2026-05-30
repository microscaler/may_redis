// TLS configuration types for may-redis.
//
// Provides TlsVersion, RustlsRootCerts, and ClientCerts — the building blocks
// for encrypted Redis connections.

use std::io;
use std::path::PathBuf;

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
    /// Returns [`super::TlsError::InvalidTlsVersion`] if the string is not "1.2" or "1.3".
    pub fn from_str(s: &str) -> Result<Self, super::TlsError> {
        match s.trim() {
            "1.2" => Ok(Self::Tls12),
            "1.3" => Ok(Self::Tls13),
            _ => Err(super::TlsError::InvalidTlsVersion(format!(
                "unsupported TLS version: {s} (expected '1.2' or '1.3')"
            ))),
        }
    }
}

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
    /// Returns [`super::TlsError::Config`] if PEM files cannot be loaded.
    pub fn to_root_store(&self) -> Result<rustls::RootCertStore, super::TlsError> {
        let mut store = rustls::RootCertStore::empty();

        match self {
            Self::WebPkiRoots => {
                for ta in webpki_roots::TLS_SERVER_ROOTS {
                    store.roots.push(rustls::pki_types::TrustAnchor {
                        subject: rustls::pki_types::Der::from_slice(ta.subject),
                        subject_public_key_info: rustls::pki_types::Der::from_slice(ta.spki),
                        name_constraints: ta.name_constraints.map(|nc| rustls::pki_types::Der::from_slice(nc)),
                    });
                }
            }
            Self::Pem(paths) => {
                for path in paths {
                    let file = std::fs::File::open(path).map_err(|e| {
                        super::TlsError::Config(format!(
                            "failed to open CA cert file {}: {e}",
                            path.display()
                        ))
                    })?;
                    let mut reader = io::BufReader::new(file);
                    let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
                        rustls_pemfile::certs(&mut reader)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| {
                                super::TlsError::Config(format!(
                                    "failed to parse PEM certs from {}: {e}",
                                    path.display()
                                ))
                            })?;
                    store.add_parsable_certificates(certs);
                }
            }
            Self::Der(certs) => {
                for cert in certs {
                    store.add_parsable_certificates(vec![rustls::pki_types::CertificateDer::from(
                        cert.as_slice(),
                    )]);
                }
            }
        }

        Ok(store)
    }
}

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
    pub fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, super::TlsError> {
        let certs: Vec<rustls::pki_types::CertificateDer<'static>> = rustls_pemfile::certs(&mut &cert_pem[..])
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                super::TlsError::Config(format!("failed to parse client certificate PEM: {e}"))
            })?;

        let key: rustls::pki_types::PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut &key_pem[..])
            .map_err(|e| super::TlsError::Config(format!("failed to parse private key PEM: {e}")))?
            .ok_or_else(|| {
                super::TlsError::Config("no private key found in PEM data".to_string())
            })?;

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
