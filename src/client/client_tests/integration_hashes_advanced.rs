#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::HashesCommands;


// ---------------------------------------------------------------------------
// HINCRBY — Atomic field increment
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hincrby() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: i64 = client.execute(client.hincrby("hash10", "counter", 5)).unwrap();
        assert_eq!(result, 5);

        let result: i64 = client.execute(client.hincrby("hash10", "counter", 3)).unwrap();
        assert_eq!(result, 8);

        let result: i64 = client.execute(client.hincrby("hash10", "counter", -2)).unwrap();
        assert_eq!(result, 6);

        let value: Option<String> = client.execute(client.hget("hash10", "counter")).unwrap();
        assert_eq!(value, Some("6".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// HSCAN — Incremental iteration (no pattern)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hscan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..100 {
            client.execute(client.hset(format!("hash11"), format!("field_{i}"), format!("val_{i}"))).ok();
        }

        let result: (i64, Vec<String>) = client.execute(client.hscan("hash11", 0)).unwrap();
        let (next_cursor, items) = result;
        assert!(next_cursor >= 0, "HSCAN should return cursor");
        assert!(!items.is_empty(), "HSCAN should return at least some fields");

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_hscan_match() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..50 {
            client.execute(client.hset("hash12", format!("prefix_a_{i}"), format!("val_{i}"))).ok();
            client.execute(client.hset("hash12", format!("prefix_b_{i}"), format!("val_{i}"))).ok();
        }

        let result: (i64, Vec<String>) = client.execute(client.hscan_match("hash12", 0, "prefix_a_*")).unwrap();
        let (next_cursor, items) = result;
        assert!(next_cursor >= 0);

        for item in &items {
            assert!(item.starts_with("prefix_a_"), "HSCAN_MATCH should only return prefix_a_* fields");
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Hash on non-existent key — behavior checks
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_empty_key() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let hget: Option<String> = client.execute(client.hget("nonexistent", "field")).unwrap();
        assert!(hget.is_none());

        let hgetall: Vec<(String, String)> = client.execute(client.hgetall("nonexistent")).unwrap();
        assert!(hgetall.is_empty());

        let hkeys: Vec<String> = client.execute(client.hkeys("nonexistent")).unwrap();
        assert!(hkeys.is_empty());

        let hvals: Vec<String> = client.execute(client.hvals("nonexistent")).unwrap();
        assert!(hvals.is_empty());

        let hlen: i64 = client.execute(client.hlen("nonexistent")).unwrap();
        assert_eq!(hlen, 0);

        let hexists: i64 = client.execute(client.hexists("nonexistent", "field")).unwrap();
        assert_eq!(hexists, 0);

        let hdel: i64 = client.execute(client.hdel("nonexistent", "field")).unwrap();
        assert_eq!(hdel, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Large hash — 1000 fields
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_large() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..1000 {
            client.execute(client.hset("hash_large", format!("field_{i}"), format!("value_{i}"))).ok();
        }

        let len: i64 = client.execute(client.hlen("hash_large")).unwrap();
        assert_eq!(len, 1000);

        let value: Option<String> = client.execute(client.hget("hash_large", "field_999")).unwrap();
        assert_eq!(value, Some("value_999".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Hash — type mismatch error (HSET on a string key)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_wrong_type() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.set("str_key", "not_a_hash")).unwrap();

        let result: Result<i64, _> = client.execute(client.hset("str_key", "field", "value"));
        assert!(result.is_err(), "HSET on string key should fail");

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Hash — pipeline with multiple hash commands
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_pipeline() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let mut pipeline = client.pipeline();
        pipeline.add(client.hset("hash_pipe", "name", "alice"));
        pipeline.add(client.hset("hash_pipe", "age", "30"));
        pipeline.add(client.hset("hash_pipe", "city", "NYC"));
        pipeline.add(client.hget("hash_pipe", "name"));
        pipeline.add(client.hlen("hash_pipe"));

        let results: (i64, i64, i64, Option<String>, i64) = pipeline.execute().unwrap();
        assert_eq!(results.0, 1);
        assert_eq!(results.1, 1);
        assert_eq!(results.2, 1);
        assert_eq!(results.3, Some("alice".to_string()));
        assert_eq!(results.4, 3);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Hash — concurrent access
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_hash_concurrent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute(client.hset("hash_concurrent", "worker_a", "data_a")).ok();
        c2.execute(client.hset("hash_concurrent", "worker_b", "data_b")).ok();

        let v1: Option<String> = c1.execute(client.hget("hash_concurrent", "worker_a")).unwrap();
        let v2: Option<String> = c2.execute(client.hget("hash_concurrent", "worker_b")).unwrap();

        assert_eq!(v1, Some("data_a".to_string()));
        assert_eq!(v2, Some("data_b".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

