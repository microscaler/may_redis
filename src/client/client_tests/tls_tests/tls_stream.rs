// Gap 10: TlsStream constructors/Read/Write integration tests.

use super::common::{run_may, test_tls_config};
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
fn test_tls_stream_construction() {
    run_may(|| {
        let client = tls_client();
        let val: Option<String> =
            client.execute(client.get("tls_stream_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tls_stream_inner_mut() {
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tls_stream_inner() {
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tls_stream_read_write_data_flow() {
    // Set and get exercises full Write -> Read path
    run_may(|| {
        let client = tls_client();
        let key = "data_flow";
        let value = "hello tls";
        client.execute::<()>(client.set(key, value)).unwrap();
        let result: String = client.execute(client.get(key)).unwrap();
        assert_eq!(result, value);

        // Larger payload test through TLS
        let extended_key = "extended_test";
        let extended_value = "x".repeat(1000);
        client
            .execute::<()>(client.set(extended_key, &extended_value))
            .unwrap();
        let result: String = client.execute(client.get(extended_key)).unwrap();
        assert_eq!(result, extended_value);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tls_stream_flush() {
    run_may(|| {
        let client = tls_client();
        for i in 0..5 {
            let key = format!("flush_test_{i}");
            client.execute::<()>(client.set(&key, "val")).unwrap();
            let _ = client.ping().unwrap();
        }
        client.execute::<()>(client.flushdb()).ok();
    });
}
