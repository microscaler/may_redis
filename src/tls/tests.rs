#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::config::{TlsVersion, RustlsRootCerts};
    use super::connector::{TlsConfig, TlsError};

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
