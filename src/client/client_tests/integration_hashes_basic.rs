#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::HashesCommands;


// ---------------------------------------------------------------------------
// HSET / HGET — Single field store and retrieve
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hset_hget() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // HSET — single field
        let result: i64 = client.execute(client.hset("hash1", "name", "alice")).unwrap();
        assert_eq!(result, 1, "HSET on new field should return 1");

        // HGET — retrieve the field
        let value: Option<String> = client.execute(client.hget("hash1", "name")).unwrap();
        assert_eq!(value, Some("alice".to_string()));

        // HSET — overwrite existing field
        let result: i64 = client.execute(client.hset("hash1", "name", "bob")).unwrap();
        assert_eq!(result, 0, "HSET on existing field should return 0");

        let value: Option<String> = client.execute(client.hget("hash1", "name")).unwrap();
        assert_eq!(value, Some("bob".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HMSET / HGET — Multi-field set and single field get
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hmset_hget() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let fields: Vec<(&str, &str)> = vec![("name", "alice"), ("age", "30"), ("city", "NYC")];
        let result: i64 = client.execute(client.hmset("hash2", &fields)).unwrap();
        assert_eq!(result, 3, "HMSET should set all fields");

        let name: Option<String> = client.execute(client.hget("hash2", "name")).unwrap();
        assert_eq!(name, Some("alice".to_string()));

        let age: Option<String> = client.execute(client.hget("hash2", "age")).unwrap();
        assert_eq!(age, Some("30".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HGETALL — Retrieve all fields and values
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hgetall() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash3", "f1", "v1")).ok();
        client.execute(client.hset("hash3", "f2", "v2")).ok();

        let result: Vec<(String, String)> = client.execute(client.hgetall("hash3")).unwrap();
        assert_eq!(result.len(), 2, "HGETALL should return 2 fields");

        let map: std::collections::HashMap<_, _> = result.into_iter().collect();
        assert_eq!(map.get("f1"), Some(&"v1".to_string()));
        assert_eq!(map.get("f2"), Some(&"v2".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HDEL — Delete single and multiple fields
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hdel_single() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash4", "a", "1")).ok();
        client.execute(client.hset("hash4", "b", "2")).ok();

        let result: i64 = client.execute(client.hdel("hash4", "a")).unwrap();
        assert_eq!(result, 1, "HDEL existing field should return 1");

        let value: Option<String> = client.execute(client.hget("hash4", "a")).unwrap();
        assert!(value.is_none(), "Deleted field should be None");

        let result: i64 = client.execute(client.hdel("hash4", "missing")).unwrap();
        assert_eq!(result, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hdel_fields() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash5", "a", "1")).ok();
        client.execute(client.hset("hash5", "b", "2")).ok();
        client.execute(client.hset("hash5", "c", "3")).ok();

        let fields: Vec<&str> = vec!["a", "b"];
        let result: i64 = client.execute(client.hdel_fields("hash5", &fields)).unwrap();
        assert_eq!(result, 2, "HDEL_FIELDS should delete 2 fields");

        let value: Option<String> = client.execute(client.hget("hash5", "a")).unwrap();
        assert!(value.is_none());

        let value: Option<String> = client.execute(client.hget("hash5", "c")).unwrap();
        assert_eq!(value, Some("3".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HKEYS / HVALS — Get field names and values
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hkeys() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash6", "x", "1")).ok();
        client.execute(client.hset("hash6", "y", "2")).ok();
        client.execute(client.hset("hash6", "z", "3")).ok();

        let keys: Vec<String> = client.execute(client.hkeys("hash6")).unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"x".to_string()));
        assert!(keys.contains(&"y".to_string()));
        assert!(keys.contains(&"z".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hvals() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash7", "a", "val_a")).ok();
        client.execute(client.hset("hash7", "b", "val_b")).ok();

        let vals: Vec<String> = client.execute(client.hvals("hash7")).unwrap();
        assert_eq!(vals.len(), 2);
        assert!(vals.contains(&"val_a".to_string()));
        assert!(vals.contains(&"val_b".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HLEN / HEXISTS — Field count and existence check
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hlen() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash8", "k1", "v1")).ok();
        client.execute(client.hset("hash8", "k2", "v2")).ok();

        let len: i64 = client.execute(client.hlen("hash8")).unwrap();
        assert_eq!(len, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hexists() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.hset("hash9", "field", "value")).ok();

        let exists: i64 = client.execute(client.hexists("hash9", "field")).unwrap();
        assert_eq!(exists, 1);

        let exists: i64 = client.execute(client.hexists("hash9", "missing")).unwrap();
        assert_eq!(exists, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

