#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::tls::config::{RustlsRootCerts, TlsVersion};
    use crate::tls::connector::{TlsConfig, TlsError};

    // -----------------------------------------------------------------------
    // TLS Version tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tls_version_from_str_12() {
        assert_eq!(TlsVersion::parse("1.2").unwrap(), TlsVersion::Tls12);
    }

    #[test]
    fn test_tls_version_from_str_13() {
        assert_eq!(TlsVersion::parse("1.3").unwrap(), TlsVersion::Tls13);
    }

    #[test]
    fn test_tls_version_from_str_invalid() {
        let err = TlsVersion::parse("1.1").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_from_str_empty() {
        let err = TlsVersion::parse("").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_ordering() {
        assert!(TlsVersion::Tls12 < TlsVersion::Tls13);
    }

    // -----------------------------------------------------------------------
    // TlsConfig tests
    // -----------------------------------------------------------------------

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

    #[test]
    fn test_tls_config_min_max_same() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            min_version: TlsVersion::Tls13,
            max_version: TlsVersion::Tls13,
            ..TlsConfig::default()
        };
        assert!(config.into_config().is_ok());
    }

    #[test]
    fn test_tls_config_server_name() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            server_name: "redis.example.com".to_string(),
            ..TlsConfig::default()
        };
        assert_eq!(config.server_name, "redis.example.com");
        assert!(config.into_config().is_ok());
    }

    #[test]
    fn test_tls_config_no_verify_server() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            verify_server: false,
            ..TlsConfig::default()
        };
        assert!(config.into_config().is_ok());
    }

    #[test]
    fn test_tls_config_no_client_certs() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            client_certs: None,
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: true,
        };
        assert!(config.into_config().is_ok());
    }

    // -----------------------------------------------------------------------
    // Root Certificate Store tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rustls_root_certs_webpki() {
        let certs = RustlsRootCerts::WebPkiRoots;
        let store = certs.to_root_store();
        assert!(store.is_ok());
        assert!(!store.unwrap().roots.is_empty());
    }

    // -----------------------------------------------------------------------
    // ClientCerts tests (Story 14.2)
    // -----------------------------------------------------------------------

    #[test]
    fn test_client_certs_from_der() {
        let certs = crate::tls::ClientCerts::from_der(
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![7, 8, 9],
        );
        assert_eq!(certs.certificates.len(), 2);
        assert_eq!(certs.private_key, vec![7, 8, 9]);
    }

    #[test]
    fn test_client_certs_from_pem_invalid() {
        let result = crate::tls::ClientCerts::from_pem(b"not a cert", b"not a key");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("certificate") || err.contains("private key"),
            "expected cert/key error, got: {err}"
        );
    }

    /// Test: TlsConfig with client_certs builds a rustls ClientConfig.
    /// With dummy DER data, into_config() should fail gracefully with a
    /// client-certificate-related error.
    #[test]
    fn test_tls_config_mtls_from_der() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            client_certs: Some(crate::tls::ClientCerts {
                certificates: vec![vec![0u8; 1]],
                private_key: vec![0u8; 1],
            }),
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: true,
        };
        let result = config.into_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("client cert") || err.contains("invalid"),
            "expected client-cert related error, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // TlsError tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tls_error_client_cert_required_display() {
        let err = TlsError::ClientCertRequired("server requested cert".to_string());
        let display = err.to_string();
        assert!(display.contains("Client certificate required"));
    }

    #[test]
    fn test_tls_error_config_display() {
        let err = TlsError::Config("bad config".to_string());
        assert!(err.to_string().contains("TLS config error"));
    }

    #[test]
    fn test_tls_error_handshake_display() {
        let err = TlsError::Handshake("conn refused".to_string());
        assert!(err.to_string().contains("TLS handshake error"));
    }

    #[test]
    fn test_tls_error_verification_display() {
        let err = TlsError::Verification("expired".to_string());
        assert!(err.to_string().contains("verification"));
    }

    #[test]
    fn test_tls_error_handshake_timeout_display() {
        let err = TlsError::HandshakeTimeout;
        assert!(err.to_string().contains("timed out"));
    }
}
