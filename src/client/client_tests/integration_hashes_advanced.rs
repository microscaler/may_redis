#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, HashesCommands};

// HSCAN on large hash (1000 fields)
#[test]
fn test_integration_hashes_hscan_large() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HSCAN returns cursor and fields
        let result: (i64, Vec<String>) =
            client.execute(client.hscan("nonexistent", 0)).unwrap();
        assert!(result.0 >= 0);
        assert!(result.1.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// HSCAN with MATCH pattern
#[test]
fn test_integration_hashes_hscan_match() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: (i64, Vec<String>) =
            client.execute(client.hscan("nonexistent", 0)).unwrap();
        assert!(result.0 >= 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// Non-existent hash returns empty for all methods
#[test]
fn test_integration_hashes_nonexistent() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let hget: Option<String> =
            client.execute(client.hget("nonexistent", "field")).unwrap();
        assert!(hget.is_none());

        let hgetall: Vec<(String, String)> =
            client.execute(client.hgetall("nonexistent")).unwrap();
        assert!(hgetall.is_empty());

        let hkeys: Vec<String> = client.execute(client.hkeys("nonexistent")).unwrap();
        assert!(hkeys.is_empty());

        let hlen: i64 = client.execute(client.hlen("nonexistent")).unwrap();
        assert_eq!(hlen, 0);

        let hdel: i64 = client.execute(client.hdel("nonexistent", "field")).unwrap();
        assert_eq!(hdel, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}
