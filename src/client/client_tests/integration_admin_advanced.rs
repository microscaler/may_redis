#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, StringsCommands};

// FLUSHALL — Delete all keys from all databases
#[test]
fn test_integration_admin_flushall() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();
        client.execute::<()>(client.set("db0_key", "value")).ok();

        client.execute::<()>(client.select(1)).ok();
        client.execute::<()>(client.flushdb()).ok();
        client.execute::<()>(client.set("db1_key", "value")).ok();

        client.execute::<()>(client.select(0)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 1);

        client.execute::<()>(client.select(1)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 1);

        client.execute::<()>(client.select(0)).ok();
        client.execute::<()>(client.flushall()).ok();

        client.execute::<()>(client.select(0)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 0);

        client.execute::<()>(client.select(1)).ok();
        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 0);

        client.execute::<()>(client.select(0)).ok();
        client.execute::<()>(client.flushall()).ok();
    });
}

// SORT — Sort a list
#[test]
fn test_integration_admin_sort_list() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // SORT on a string key returns WRONGTYPE
        client.execute::<()>(client.set("sort_string", "abc")).ok();
        let result: Result<Vec<String>, _> = client.execute(client.sort("sort_string"));
        assert!(result.is_err(), "SORT on string key should fail");
        assert!(
            result.unwrap_err().to_string().contains("WRONGTYPE"),
            "expected WRONGTYPE error"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SCAN with MATCH pattern
#[test]
fn test_integration_admin_scan_match() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..30 {
            client
                .execute::<()>(client.set(format!("user:{i}"), format!("val_{i}")))
                .ok();
            client
                .execute::<()>(client.set(format!("session:{i}"), format!("s_{i}")))
                .ok();
        }

        let result: (i64, Vec<String>) =
            client.execute(client.scan_match(0, "user:*")).unwrap();
        let (_cursor, keys) = result;
        assert!(!keys.is_empty());
        for key in &keys {
            assert!(key.starts_with("user:"));
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}

// INFO — Get server info
#[test]
fn test_integration_admin_info() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let info: String = client.execute(client.info_section("server")).unwrap();
        assert!(!info.is_empty());
        assert!(info.contains("redis_version") || info.contains("Server"));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// Admin — concurrent access with key operations
#[test]
fn test_integration_admin_concurrent() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute::<()>(c1.set("admin_concurrent:a", "val1")).ok();
        c2.execute::<()>(c2.set("admin_concurrent:b", "val2")).ok();

        let type_a: String =
            client.execute(client.type_("admin_concurrent:a")).unwrap();
        assert_eq!(type_a.to_lowercase(), "string");

        let type_b: String =
            client.execute(client.type_("admin_concurrent:b")).unwrap();
        assert_eq!(type_b.to_lowercase(), "string");

        client.execute::<()>(client.flushdb()).ok();
    });
}
