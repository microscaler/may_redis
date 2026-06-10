#![allow(clippy::module_inception)]
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
    fn test_tls_version_from_str_whitespace() {
        assert_eq!(TlsVersion::parse(" 1.2 ").unwrap(), TlsVersion::Tls12);
        assert_eq!(TlsVersion::parse("1.3  ").unwrap(), TlsVersion::Tls13);
    }

    #[test]
    fn test_tls_version_from_str_non_standard() {
        let err = TlsVersion::parse("1.3.1").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_from_str_with_prefix() {
        let err = TlsVersion::parse("v1.2").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_from_str_whitespace_only() {
        let err = TlsVersion::parse("   ").unwrap_err();
        assert!(matches!(err, TlsError::InvalidTlsVersion(_)));
    }

    #[test]
    fn test_tls_version_to_supported() {
        assert_eq!(TlsVersion::Tls12.to_supported(), &rustls::version::TLS12);
        assert_eq!(TlsVersion::Tls13.to_supported(), &rustls::version::TLS13);
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

    #[test]
    fn test_rustls_root_certs_pem_nonexistent_file() {
        let certs = RustlsRootCerts::Pem(vec![std::path::PathBuf::from(
            "/nonexistent/path/to/cert.pem",
        )]);
        let result = certs.to_root_store();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("CA cert file") || err.contains("failed to open"),
            "expected file-open error, got: {err}"
        );
    }

    #[test]
    fn test_rustls_root_certs_pem_invalid_pem_content() {
        // Create a temp file with invalid PEM content
        let dir = std::env::temp_dir();
        let path = dir.join("may-redis-test-invalid-cert.pem");
        std::fs::write(&path, "this is not a valid pem file").unwrap();
        let certs = RustlsRootCerts::Pem(vec![path]);
        // Negative: a PEM file yielding zero usable certificates is a
        // configuration error, not a silent empty trust store.
        let err = certs
            .to_root_store()
            .expect_err("PEM file without certificates must fail");
        assert!(
            err.to_string().contains("certificate"),
            "error must mention certificates, got: {err}"
        );
    }

    #[test]
    fn test_rustls_root_certs_der_empty_vec() {
        let certs = RustlsRootCerts::Der(vec![]);
        let store = certs.to_root_store();
        assert!(store.is_ok());
        assert!(store.unwrap().roots.is_empty());
    }

    /// Negative: garbage DER must be rejected loudly. rustls ignores
    /// unparsable certs, which previously left an empty trust store and a
    /// cryptic handshake failure much later.
    #[test]
    fn test_rustls_root_certs_der_garbage_is_error() {
        let certs = RustlsRootCerts::Der(vec![vec![0u8; 100]]);
        let store = certs.to_root_store();
        let err = store.expect_err("all-garbage DER input must fail");
        assert!(
            err.to_string().contains("parsable"),
            "error must explain that nothing was parsable, got: {err}"
        );
    }

    /// Positive: a real DER certificate loads into a non-empty store.
    #[test]
    fn test_rustls_root_certs_der_valid_cert() {
        let ca_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/tls/ca.crt");
        let pem = std::fs::read(&ca_path).expect("read tests/tls/ca.crt");
        let ders: Vec<Vec<u8>> = rustls_pemfile::certs(&mut &pem[..])
            .collect::<Result<Vec<_>, _>>()
            .expect("parse ca.crt PEM")
            .into_iter()
            .map(|c| c.to_vec())
            .collect();
        assert!(!ders.is_empty(), "fixture CA cert must contain a cert");

        let store = RustlsRootCerts::Der(ders).to_root_store();
        assert!(!store.expect("valid DER must load").roots.is_empty());
    }

    #[test]
    fn test_rustls_root_certs_clone() {
        let certs1 = RustlsRootCerts::WebPkiRoots;
        // RustlsRootCerts is Copy — clone is a no-op, but verify no panic.
        let certs2 = certs1;
        let _ = certs2.to_root_store();
    }

    #[test]
    fn test_client_certs_from_pem_valid_cert_no_key() {
        // Valid cert PEM but no key — should fail with "no private key" error.
        let cert_pem = b"-----BEGIN CERTIFICATE-----\n".to_vec().into_boxed_slice();
        let result = crate::tls::ClientCerts::from_pem(&cert_pem, b"not a key");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("private key") || err.contains("certificate"),
            "expected cert/key error, got: {err}"
        );
    }

    #[test]
    fn test_client_certs_from_pem_empty_cert() {
        let result = crate::tls::ClientCerts::from_pem(b"", b"not a key");
        assert!(result.is_err());
    }

    #[test]
    fn test_client_certs_clone() {
        let certs = crate::tls::ClientCerts {
            certificates: vec![vec![1, 2, 3]],
            private_key: vec![4, 5, 6],
        };
        let certs2 = certs.clone();
        assert_eq!(certs.certificates, certs2.certificates);
        assert_eq!(certs.private_key, certs2.private_key);
    }

    // -----------------------------------------------------------------------
    // TlsConfig min/max=1.2 only path
    // -----------------------------------------------------------------------

    #[test]
    fn test_tls_config_min_max_tls12_only() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::WebPkiRoots,
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls12,
            ..TlsConfig::default()
        };
        assert!(config.into_config().is_ok());
    }

    #[test]
    fn test_tls_config_pem_with_valid_ca_cert() {
        // Use a real self-signed CA cert to exercise the Pem path through into_config()
        let ca_path = std::path::PathBuf::from("/tmp/test_ca_cert.pem");
        if !ca_path.exists() {
            // If the test cert doesn't exist, skip this test
            // (it's only generated in CI/CD or when manually run)
            return;
        }
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![ca_path]),
            client_certs: None,
            server_name: "may-redis-test".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: false, // self-signed, don't verify
        };
        // The PEM cert should parse successfully into a rustls ClientConfig
        assert!(config.into_config().is_ok());
    }

    #[test]
    fn test_tls_config_pem_multiple_certs() {
        let ca_path = std::path::PathBuf::from("/tmp/test_ca_cert.pem");
        if !ca_path.exists() {
            return;
        }
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![ca_path.clone(), ca_path]),
            client_certs: None,
            server_name: "may-redis-test".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: false,
        };
        // Multiple PEM certs should build successfully
        assert!(config.into_config().is_ok());
    }

    /// Positive: with verification disabled, no root certificates are
    /// needed — the skip verifier must not require trust anchors.
    #[test]
    fn test_tls_config_pem_empty_path() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![]),
            client_certs: None,
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: false,
        };
        // Empty PEM paths produce an empty root store (no error)
        assert!(config.into_config().is_ok());
    }

    /// Negative: verification enabled with an empty trust store must fail
    /// at config time — previously it built fine and every handshake then
    /// failed with an inscrutable UnknownIssuer error.
    #[test]
    fn test_tls_config_verify_with_empty_roots_is_error() {
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![]),
            client_certs: None,
            server_name: "localhost".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: true,
        };
        let err = config
            .into_config()
            .expect_err("verify_server with no roots must fail");
        assert!(
            err.to_string().contains("root certificate"),
            "error must explain the missing roots, got: {err}"
        );
    }

    #[test]
    fn test_tls_config_mtls_from_pem() {
        let ca_path = std::path::PathBuf::from("/tmp/test_ca_cert.pem");
        let key_path = std::path::PathBuf::from("/tmp/test_ca_key.pem");
        if !ca_path.exists() || !key_path.exists() {
            return;
        }
        let cert_bytes = std::fs::read(&ca_path).unwrap();
        let key_bytes = std::fs::read(&key_path).unwrap();
        let certs = crate::tls::ClientCerts::from_pem(&cert_bytes, &key_bytes);
        let config = TlsConfig {
            root_certificates: RustlsRootCerts::Pem(vec![ca_path]),
            client_certs: Some(certs.unwrap()),
            server_name: "may-redis-test".to_string(),
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_server: false,
        };
        // mTLS config with real PEM cert and key should build successfully
        assert!(config.into_config().is_ok());
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
