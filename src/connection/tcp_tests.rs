#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::connection::tcp::{resolve, ConnectionError, TcpConnector};

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
        let result = TcpConnector::connect_url_timeout("redis://nonexistent.invalid:6379", 1);
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
    #[ignore = "requires live network namespace"]
    fn test_connect_refused_returns_connect() {
        use may::go;

        let wrapper = std::sync::Mutex::new(None::<()>);
        let _wrapper2 = wrapper.lock().unwrap();
        let wrapper2 = std::sync::Arc::new(std::sync::Mutex::new(None::<()>));
        let wrapper3 = std::sync::Arc::clone(&wrapper2);

        let _ = go!(move || {
            let result = TcpConnector::connect_timeout("127.0.0.1", 1, 5);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ConnectionError::Connect(_)));
            *wrapper3.lock().unwrap() = Some(());
        });
    }

    #[test]
    fn test_connect_default_timeout() {
        let _ = TcpConnector::connect("127.0.0.1", 6379);
    }
}
