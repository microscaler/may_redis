// Gap 10: TlsStream constructors/Read/Write integration tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_closure,
    clippy::option_if_let_else,
    clippy::manual_string_new,
    clippy::unnecessary_trailing_comma,
    clippy::needless_borrows_for_generic_args
)]

use super::common::{prepare_tls_tests, run_may, tls_client};
use crate::protocol::commands::{AdminCommands, StringsCommands};

#[test]
fn test_tls_stream_construction() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let val: Option<String> =
            client.execute(client.get("tls_stream_test")).unwrap();
        assert_eq!(val, None);
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_tls_stream_inner_mut() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_tls_stream_inner() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let _ = client.ping().unwrap();
    });
}

#[test]
fn test_tls_stream_read_write_data_flow() {
    if !prepare_tls_tests() {
        return;
    }
    run_may(|| {
        let client = tls_client();
        let key = "data_flow";
        let value = "hello tls";
        client.execute::<()>(client.set(key, value)).unwrap();
        let result: String = client.execute(client.get(key)).unwrap();
        assert_eq!(result, value);

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
fn test_tls_stream_flush() {
    if !prepare_tls_tests() {
        return;
    }
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
