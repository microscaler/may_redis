#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::AdminCommands;

// ---------------------------------------------------------------------------
// FLUSHALL — Delete all keys from all databases
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_flushall() {
    run_may(|| {
        let client = shared_client();
        // Create keys in DB 0 and DB 1
        client.execute::<()>(client.flushdb()).ok();
        client.execute(client.set("db0_key", "value")).ok();

        client.execute::<()>(client.select(1)).ok();
        client.execute::<()>(client.flushdb()).ok();
        client.execute(client.set("db1_key", "value")).ok();

        // DB 0 should have 1 key
        client.execute::<()>(client.select(0)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 1);

        // Switch to DB 1
        client.execute::<()>(client.select(1)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 1);

        // FLUSHALL should clear all databases
        client.execute::<()>(client.select(0)).ok();
        client.execute::<()>(client.flushall()).ok();

        client.execute::<()>(client.select(0)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 0);

        client.execute::<()>(client.select(1)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 0);

        // Cleanup
        client.execute::<()>(client.select(0)).ok();
        client.execute::<()>(client.flushall()).ok();
    });
}

// ---------------------------------------------------------------------------
// SORT — Sort a list
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_sort_list() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.lpush("sort_list", "3")).ok();
        client.execute(client.lpush("sort_list", "1")).ok();
        client.execute(client.lpush("sort_list", "2")).ok();

        let sorted: Vec<String> = client.execute(client.sort("sort_list")).unwrap();
        assert_eq!(sorted, vec!["1", "2", "3"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SORT with LIMIT
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_sort_limit() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..10 {
            client.execute(client.lpush("sort_limit", format!("{}", i))).ok();
        }

        // Get just first 3 sorted items
        let limited: Vec<String> = client
            .execute(client.sort_limit("sort_limit", 0, 3))
            .unwrap();
        assert_eq!(limited.len(), 3);
        assert_eq!(limited, vec!["0", "1", "2"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SCAN with MATCH pattern
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_scan_match() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..30 {
            client
                .execute(client.set(format!("user:{}", i), format!("val_{}", i)))
                .ok();
            client
                .execute(client.set(format!("session:{}", i), format!("s_{}", i)))
                .ok();
        }

        // Scan for user:* keys only
        let result: (i64, Vec<String>) = client
            .execute(client.scan_match(0, "user:*"))
            .unwrap();
        let (_cursor, keys) = result;
        assert!(!keys.is_empty());
        for key in &keys {
            assert!(
                key.starts_with("user:"),
                "SCAN_MATCH should only return user:* keys, got: {}",
                key
            );
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// INFO — Get server info
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_info() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // INFO server should return server info block
        let info: String = client.execute(client.info_section("server")).unwrap();
        assert!(!info.is_empty(), "INFO server should return data");

        // Should contain server version or similar
        assert!(
            info.contains("redis_version") || info.contains("Server"),
            "INFO server should contain server info"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// CONFIG GET — Get config parameters
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_config_get() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // GET maxmemory should return config value
        let config: Vec<String> = client.execute(client.config_get("maxmemory")).unwrap();
        assert!(
            !config.is_empty(),
            "CONFIG GET maxmemory should return config"
        );

        // Config returns [key, value] pairs
        assert!(
            config.len() >= 2,
            "CONFIG GET should return at least key and value"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Admin — concurrent access with key operations
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_admin_concurrent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute(c1.set("admin_concurrent:a", "val1")).ok();
        c2.execute(c2.set("admin_concurrent:b", "val2")).ok();

        let type_a: String = client.execute(client.type_("admin_concurrent:a")).unwrap();
        let type_b: String = client.execute(client.type_("admin_concurrent:b")).unwrap();
        assert_eq!(type_a.to_lowercase(), "string");
        assert_eq!(type_b.to_lowercase(), "string");

        client.execute::<()>(client.flushdb()).ok();
    });
}
