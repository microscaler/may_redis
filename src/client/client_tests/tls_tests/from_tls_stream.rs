// Gap 4: from_tls_stream() integration tests.

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
fn test_from_tls_stream_defaults() {
    // Scenario 1: Connection has correct defaults
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("from_tls_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_from_tls_stream_with_ssrf_none() {
    // Scenario 2: Default connect_tls() has ssrf_config=None
    run_may(|| {
        let client = tls_client();
        let val: Option<String> = client.execute(client.get("ssrf_none_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_from_tls_stream_with_ssrf_some() {
    // Scenario 3: connect_tls_with_ssrf creates connection with ssrf_config=Some
    run_may(|| {
        let config = test_tls_config();
        let ssrf = SsrfConfig {
            deny_private: false,
            deny_link_local: false,
            deny_loopback: false,
        };
        let client =
            RedisClient::connect_tls_with_ssrf("127.0.0.1", TLS_PORT, &config, 5, ssrf)
                .expect("TLS connection with SSRF should succeed");
        let val: Option<String> = client.execute(client.get("ssrf_some_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_from_tls_stream_connection_loop_starts() {
    // Scenario 4: Background loop starts and responds
    run_may(|| {
        let client = tls_client();
        for i in 0..5 {
            let key = format!("loop_test_{i}");
            client.execute::<()>(client.set(&key, "val")).unwrap();
            let val: String = client.execute(client.get(&key)).unwrap();
            assert_eq!(val, "val");
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}
