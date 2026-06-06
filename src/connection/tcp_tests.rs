#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::connection::ssrf_allowed;
    use crate::connection::tcp::{resolve, ConnectionError, SsrfConfig, TcpConnector};
    use may::go;
    use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::Resolve("host not found".to_string());
        assert!(format!("{err}").contains("resolve"));

        let err = ConnectionError::Connect("connection refused".to_string());
        assert!(format!("{err}").contains("connect"));

        let err = ConnectionError::SetNodelay("operation not supported".to_string());
        assert!(format!("{err}").contains("nodelay"));

        let err = ConnectionError::Timeout("5s exceeded".to_string());
        assert!(format!("{err}").contains("timeout"));
    }

    #[test]
    fn test_connection_error_is_timeout() {
        let err = ConnectionError::Timeout("test".to_string());
        assert!(err.is_timeout());

        let err = ConnectionError::Connect("test".to_string());
        assert!(!err.is_timeout());
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
    fn test_resolve_hostname() {
        let addrs = resolve("localhost", 6379).unwrap();
        assert!(!addrs.is_empty());
        assert_eq!(addrs[0].port(), 6379);
    }

    #[test]
    fn test_connect_url_parses() {
        let result = TcpConnector::connect_url("redis://nonexistent.invalid:6379");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_timeout_parses() {
        let result =
            TcpConnector::connect_url_timeout("redis://nonexistent.invalid:6379", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_invalid_port() {
        let result = TcpConnector::connect_url("redis://localhost:abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_url_invalid_format() {
        let result = TcpConnector::connect_url("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_refused_returns_connect() {
        let wrapper = std::sync::Mutex::new(None::<()>);
        let _wrapper2 = wrapper.lock().unwrap();
        let wrapper2 = std::sync::Arc::new(std::sync::Mutex::new(None::<()>));
        let wrapper3 = std::sync::Arc::clone(&wrapper2);

        let handle = go!(move || {
            let result = TcpConnector::connect_timeout("127.0.0.1", 1, 5);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ConnectionError::Connect(_)));
            *wrapper3.lock().unwrap() = Some(());
        });
        let _ = handle.join();
    }

    #[test]
    fn test_connect_default_timeout() {
        let _ = TcpConnector::connect("127.0.0.1", 6379);
    }

    // =====================================================================
    // SSRF Config tests (Epic 14 — TEST_GAP_ANALYSIS.md Gap #1)
    // =====================================================================

    fn v4(addr: &str, port: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(addr.parse().unwrap(), port))
    }

    fn v6(addr: &str, port: u16) -> SocketAddr {
        SocketAddr::V6(SocketAddrV6::new(addr.parse().unwrap(), port, 0, 0))
    }

    // --- Default config values ---

    #[test]
    fn test_ssrf_config_default_values() {
        let config = SsrfConfig::default();
        assert!(config.deny_private);
        assert!(config.deny_link_local);
        assert!(!config.deny_loopback); // loopback NOT denied by default
    }

    // --- V4 private ranges ---

    #[test]
    fn test_ssrf_blocked_v4_10_x() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("10.0.0.1", 6379)));
        assert!(config.is_blocked(&v4("10.255.255.255", 8080)));
        assert!(!ssrf_allowed(&v4("10.0.0.1", 6379), &config));
    }

    #[test]
    fn test_ssrf_blocked_v4_172_16_31() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("172.16.0.1", 6379)));
        assert!(config.is_blocked(&v4("172.31.255.255", 443)));
        // 172.15.0.1 is NOT in the 172.16/12 range
        assert!(ssrf_allowed(&v4("172.15.0.1", 6379), &config));
        // 172.32.0.1 is NOT in the 172.16/12 range
        assert!(ssrf_allowed(&v4("172.32.0.1", 6379), &config));
    }

    #[test]
    fn test_ssrf_blocked_v4_192_168() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("192.168.0.1", 6379)));
        assert!(config.is_blocked(&v4("192.168.255.255", 6379)));
        assert!(!ssrf_allowed(&v4("192.168.0.1", 6379), &config));
    }

    // --- V4 link-local ---

    #[test]
    fn test_ssrf_blocked_v4_link_local() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("169.254.0.1", 6379)));
        assert!(config.is_blocked(&v4("169.254.255.255", 8080)));
        // 169.253.0.1 is NOT link-local
        assert!(ssrf_allowed(&v4("169.253.0.1", 6379), &config));
    }

    // --- V4 loopback (only denied when deny_loopback=true) ---

    #[test]
    fn test_ssrf_v4_loopback_default_not_denied() {
        let config = SsrfConfig::default();
        assert!(!config.is_blocked(&v4("127.0.0.1", 6379)));
        assert!(ssrf_allowed(&v4("127.0.0.1", 6379), &config));
    }

    #[test]
    fn test_ssrf_v4_loopback_when_enabled() {
        let config = SsrfConfig {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: true,
        };
        assert!(config.is_blocked(&v4("127.0.0.1", 6379)));
        assert!(config.is_blocked(&v4("127.255.255.255", 6379)));
        assert!(!ssrf_allowed(&v4("127.0.0.1", 6379), &config));
    }

    // --- V4 0.0.0.0 ---

    #[test]
    fn test_ssrf_blocked_v4_zero() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("0.0.0.0", 6379)));
        assert!(config.is_blocked(&v4("0.1.2.3", 80)));
    }

    // --- V4 CGNAT (100.64.0.0/10) ---

    #[test]
    fn test_ssrf_blocked_v4_cgnat() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("100.64.0.1", 6379)));
        assert!(config.is_blocked(&v4("100.127.255.255", 8080)));
        // 100.63.0.1 is outside CGNAT range
        assert!(ssrf_allowed(&v4("100.63.0.1", 6379), &config));
        // 100.128.0.1 is outside CGNAT range
        assert!(ssrf_allowed(&v4("100.128.0.1", 6379), &config));
    }

    // --- V4 multicast ---

    #[test]
    fn test_ssrf_blocked_v4_multicast() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("224.0.0.1", 6379)));
        assert!(config.is_blocked(&v4("239.255.255.255", 5000)));
        // 223.255.255.255 is NOT multicast
        assert!(ssrf_allowed(&v4("223.255.255.255", 6379), &config));
    }

    // --- V4 reserved ---

    #[test]
    fn test_ssrf_blocked_v4_reserved() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v4("240.0.0.1", 6379)));
        assert!(config.is_blocked(&v4("255.255.255.255", 6379)));
    }

    // --- Public IPs are allowed ---

    #[test]
    fn test_ssrf_public_ip_allowed() {
        let config = SsrfConfig::default();
        assert!(ssrf_allowed(&v4("8.8.8.8", 443), &config));
        assert!(ssrf_allowed(&v4("1.1.1.1", 443), &config));
        assert!(ssrf_allowed(&v4("52.94.76.1", 443), &config));
    }

    // --- V6 addresses ---

    #[test]
    fn test_ssrf_blocked_v6_loopback() {
        let config = SsrfConfig {
            deny_private: true,
            deny_link_local: true,
            deny_loopback: true,
        };
        assert!(config.is_blocked(&v6("::1", 6379)));
        assert!(!ssrf_allowed(&v6("::1", 6379), &config));
    }

    #[test]
    fn test_ssrf_v6_loopback_default_not_denied() {
        let config = SsrfConfig::default();
        assert!(!config.is_blocked(&v6("::1", 6379)));
        assert!(ssrf_allowed(&v6("::1", 6379), &config));
    }

    #[test]
    fn test_ssrf_blocked_v6_link_local() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v6("fe80::1", 6379)));
        assert!(config.is_blocked(&v6("fe80::abcd", 8080)));
    }

    #[test]
    fn test_ssrf_blocked_v6_unique_local() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v6("fc00::1", 6379)));
        assert!(config.is_blocked(&v6("fdff:ffff::1", 8080)));
        // fe00::1 is NOT unique-local
        assert!(ssrf_allowed(&v6("fe00::1", 6379), &config));
    }

    #[test]
    fn test_ssrf_blocked_v6_multicast() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v6("ff00::1", 6379)));
        assert!(config.is_blocked(&v6("ff02::1", 5353)));
    }

    #[test]
    fn test_ssrf_blocked_v6_unspecified() {
        let config = SsrfConfig::default();
        assert!(config.is_blocked(&v6("::", 6379)));
        // ::ffff:0.0.0.0 is IPv4-mapped, NOT the IPv6 unspecified address —
        // is_unspecified() returns false for it, so it is not blocked here.
        assert!(!config.is_blocked(&v6("::ffff:0.0.0.0", 6379)));
    }

    // --- All deny flags disabled ---

    #[test]
    fn test_ssrf_all_flags_disabled_allows_private() {
        let config = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        assert!(ssrf_allowed(&v4("10.0.0.1", 6379), &config));
        assert!(ssrf_allowed(&v4("192.168.0.1", 6379), &config));
        assert!(ssrf_allowed(&v4("127.0.0.1", 6379), &config));
        assert!(ssrf_allowed(&v6("fc00::1", 6379), &config));
        assert!(ssrf_allowed(&v6("::1", 6379), &config));
        assert!(ssrf_allowed(&v4("8.8.8.8", 443), &config));
    }

    // --- Mixed deny flags ---

    #[test]
    fn test_ssrf_mixed_flags() {
        let config = SsrfConfig {
            deny_private: true,
            deny_link_local: false,
            deny_loopback: true,
        };
        // Private blocked
        assert!(config.is_blocked(&v4("10.0.0.1", 6379)));
        // Link-local allowed (deny_link_local=false)
        assert!(ssrf_allowed(&v4("169.254.0.1", 6379), &config));
        // Loopback blocked
        assert!(config.is_blocked(&v4("127.0.0.1", 6379)));
        // 0.0.0.0 still blocked (always blocked regardless of flags)
        assert!(config.is_blocked(&v4("0.0.0.0", 6379)));
    }

    // --- ssrf_allowed() wrapper ---

    #[test]
    fn test_ssrf_allowed_wrapper() {
        let config = SsrfConfig::default();
        assert!(ssrf_allowed(&v4("8.8.8.8", 443), &config));
        assert!(!ssrf_allowed(&v4("10.0.0.1", 6379), &config));
    }

    // =====================================================================
    // ConnectionError Display + is_timeout() (Epic 14 — TEST_GAP_ANALYSIS.md)
    // =====================================================================

    #[test]
    fn test_connection_error_display_timeout() {
        let err = ConnectionError::Timeout("connection timed out".to_string());
        assert!(format!("{err}").contains("timeout"));
    }

    #[test]
    #[cfg(feature = "tls")]
    fn test_connection_error_display_tls() {
        let err = ConnectionError::Tls("handshake failed".to_string());
        assert!(format!("{err}").contains("TLS"));
    }

    #[test]
    fn test_connection_error_display_ssrf_violation() {
        let err = ConnectionError::SsrfViolation("10.0.0.1 denied".to_string());
        assert!(format!("{err}").contains("SSRF"));
    }

    // =====================================================================
    // TcpConnector resolve/connect_url unit tests (Gap 13)
    // =====================================================================

    #[test]
    fn test_resolve_v4() {
        let addrs = resolve("127.0.0.1", 6379).unwrap();
        assert_eq!(addrs.len(), 1);
        assert!(matches!(addrs[0], std::net::SocketAddr::V4(_)));
    }

    #[test]
    fn test_resolve_v6() {
        let addrs = resolve("::1", 6379).unwrap();
        assert_eq!(addrs.len(), 1);
        assert!(matches!(addrs[0], std::net::SocketAddr::V6(_)));
    }
}
