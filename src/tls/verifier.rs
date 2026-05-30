// TLS certificate verifier — skips server certificate verification.
//
// Provides `SkipVerifier`, a `ServerCertVerifier` implementation that
// accepts **any** server certificate without validation (debugging only).

use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

/// A certificate verifier that skips verification (for debugging only).
///
/// # Security
///
/// This verifier accepts **any** server certificate without validation.
/// NEVER use this in production.
#[derive(Debug)]
pub(super) struct SkipVerifier {
    pub(super) inner: std::sync::Arc<rustls::client::WebPkiServerVerifier>,
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
