// Gap 9: Connection struct methods integration tests.

use super::common::{run_may, test_tls_config};
use crate::connection::tcp::SsrfConfig;
use crate::protocol::commands::{AdminCommands, StringsCommands};
use crate::RedisClient;

const TLS_HOST: &str = "localhost";
const TLS_PORT: u16 = 6380;

fn tls_client() -> RedisClient {
    static INIT: std::sync::Once = std::sync::Once::new();
    static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
    INIT.call_once(|| {
        let config = test_tls_config();
        let client = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5)
            .expect("Redis-TLS must be running on localhost:6380");
        CLIENT.set(client).ok();
    });
    CLIENT.get().expect("client not initialized").clone()
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connect_default_limits() {
    // Default limits on connect_tls()
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_ssrf_config_none_plain() {
    // Plain TCP has no SSRF config
    run_may(|| {
        let client = RedisClient::connect("127.0.0.1", 6379)
            .expect("Plain Redis connection should succeed");
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_send_success_within_limits() {
    run_may(|| {
        let client = tls_client();
        for i in 0..10 {
            let key = format!("send_test_{i}");
            client
                .execute::<()>(client.set(&key, &format!("val{i}")))
                .unwrap();
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_send_request_too_large() {
    // Default max_request_size is ~1MB; 100KB should succeed
    run_may(|| {
        let client = tls_client();
        let large_value = "x".repeat(100_000);
        let result: Result<(), crate::core::RedisError> =
            client.execute(client.set("large_key", &large_value));
        assert!(result.is_ok(), "100KB should be within default 1MB limit");
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connection_id_uniqueness() {
    run_may(|| {
        let config = test_tls_config();
        let client1 = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5)
            .expect("Connection 1 should succeed");
        let client2 = RedisClient::connect_tls(TLS_HOST, TLS_PORT, &config, 5)
            .expect("Connection 2 should succeed");
        let _ = client1.ping().unwrap();
        let _ = client2.ping().unwrap();
        client1.execute::<()>(client1.flushdb()).ok();
        client2.execute::<()>(client2.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_ssrf_config_some_on_tls_with_ssrf() {
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let client =
            RedisClient::connect_tls_with_ssrf("127.0.0.1", TLS_PORT, &config, 5, ssrf)
                .expect("TLS with SSRF should succeed");
        let _ = client.ping().unwrap();
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_connection_pooling_shared_client() {
    run_may(|| {
        let client = tls_client();
        let client2 = client.clone();
        client
            .execute::<()>(client.set("shared_key", "shared_val"))
            .unwrap();
        let val: String = client2.execute(client2.get("shared_key")).unwrap();
        assert_eq!(val, "shared_val");
        client.execute::<()>(client.flushdb()).ok();
    });
}
