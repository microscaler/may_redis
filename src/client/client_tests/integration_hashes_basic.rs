#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::{AdminCommands, HashesCommands};

// ---------------------------------------------------------------------------
// HGET — Get a field value (from an existing hash)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hget() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Set up a hash using HMSET via AdminCommands (not available)
        // Since HSET doesn't exist, we skip this test
        // This trait method requires a pre-existing hash

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HGETALL — Get all fields and values
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hgetall() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HGETALL on empty/nonexistent hash
        let result: Vec<(String, String)> =
            client.execute(client.hgetall("nonexistent")).unwrap();
        assert!(result.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HDEL — Delete a field
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hdel() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HDEL on nonexistent key/field returns 0
        let result: i64 = client.execute(client.hdel("nonexistent", "field")).unwrap();
        assert_eq!(result, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HKEYS — Get all field names
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hkeys() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HKEYS on nonexistent hash returns empty
        let keys: Vec<String> = client.execute(client.hkeys("nonexistent")).unwrap();
        assert!(keys.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HLEN — Get number of fields
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hlen() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HLEN on nonexistent hash returns 0
        let len: i64 = client.execute(client.hlen("nonexistent")).unwrap();
        assert_eq!(len, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HSCAN — Scan hash fields
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hashes_hscan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HSCAN on empty hash
        let result: (i64, Vec<String>) =
            client.execute(client.hscan("nonexistent", 0)).unwrap();
        assert!(result.0 >= 0);
        assert!(result.1.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}
