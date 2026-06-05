#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, StringsCommands};

// TYPE — Check key type
#[test]
fn test_integration_admin_type() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("string_key", "value")).ok();
        let string_type: String = client.execute(client.type_("string_key")).unwrap();
        assert_eq!(string_type.to_lowercase(), "string");

        let missing_type: String = client.execute(client.type_("missing_key")).unwrap();
        assert_eq!(missing_type.to_lowercase(), "none");

        client.execute::<()>(client.flushdb()).ok();
    });
}

// MOVE — Move key to another DB
#[test]
fn test_integration_admin_move() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("move_key", "move_val"))
            .ok();
        let moved: i64 = client.execute(client.move_key("move_key", 1)).unwrap();
        assert_eq!(moved, 1);

        let exists: bool = client.execute(client.exists("move_key")).unwrap();
        assert!(!exists);

        client.execute::<()>(client.select(1)).ok();
        client.execute::<()>(client.flushdb()).ok();
        client.execute::<()>(client.select(0)).ok();
    });
}

// RENAME — Rename a key
#[test]
fn test_integration_admin_rename() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("old_name", "value")).ok();
        client
            .execute::<()>(client.rename("old_name", "new_name"))
            .ok();

        let value: Option<String> = client.execute(client.get("new_name")).unwrap();
        assert_eq!(value, Some("value".to_string()));

        let exists: bool = client.execute(client.exists("old_name")).unwrap();
        assert!(!exists);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// RENAMENX — Rename only if target doesn't exist
#[test]
fn test_integration_admin_renamenx() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("source", "val1")).ok();
        client.execute::<()>(client.set("dest", "val2")).ok();

        let renamed: i64 = client.execute(client.renamemx("source", "dest")).unwrap();
        assert_eq!(renamed, 0);

        let value: Option<String> = client.execute(client.get("dest")).unwrap();
        assert_eq!(value, Some("val2".to_string()));

        let exists: bool = client.execute(client.exists("source")).unwrap();
        assert!(exists);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// TOUCH — Update access time
#[test]
fn test_integration_admin_touch() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("touch1", "a")).ok();
        client.execute::<()>(client.set("touch2", "b")).ok();
        client.execute::<()>(client.set("touch3", "c")).ok();

        let keys: Vec<&str> = vec!["touch1", "touch2"];
        let touched: i64 = client.execute(client.touch(&keys)).unwrap();
        assert_eq!(touched, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// PTTL — Get TTL in milliseconds
#[test]
fn test_integration_admin_pttl() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("no_ttl", "value")).ok();
        let pttl: i64 = client.execute(client.pttl("no_ttl")).unwrap();
        assert_eq!(pttl, -1);

        client.execute::<()>(client.pexpire("ttl_key", 60000)).ok();
        let pttl: i64 = client.execute(client.pttl("ttl_key")).unwrap();
        assert!(pttl > 0 && pttl <= 60000);

        let pttl: i64 = client.execute(client.pttl("missing")).unwrap();
        assert_eq!(pttl, -2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// PEXPIRE — Set expiry in milliseconds
#[test]
fn test_integration_admin_pexpire() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("expire_key", "value")).ok();
        let expired: i64 = client.execute(client.pexpire("expire_key", 100)).unwrap();
        assert_eq!(expired, 1);

        let pttl: i64 = client.execute(client.pttl("expire_key")).unwrap();
        assert!(pttl > 0 && pttl <= 100);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// PEXPIREAT — Set expiry at unix timestamp (ms)
#[test]
fn test_integration_admin_pexpireat() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("expat_key", "value")).ok();
        let future_ms: i64 = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64)
            + 10000;
        let expired: i64 = client
            .execute(client.pexpireat("expat_key", future_ms))
            .unwrap();
        assert_eq!(expired, 1);

        let pttl: i64 = client.execute(client.pttl("expat_key")).unwrap();
        assert!(pttl > 0 && pttl <= 10000);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// PERSIST — Remove TTL from key
#[test]
fn test_integration_admin_persist() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("persist_key", "value"))
            .ok();
        client
            .execute::<()>(client.pexpire("persist_key", 100))
            .ok();

        let pttl: i64 = client.execute(client.pttl("persist_key")).unwrap();
        assert!(pttl > 0);

        let persisted: i64 = client.execute(client.persist("persist_key")).unwrap();
        assert_eq!(persisted, 1);

        let pttl: i64 = client.execute(client.pttl("persist_key")).unwrap();
        assert_eq!(pttl, -1);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SELECT — Select database
#[test]
fn test_integration_admin_select() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("key_in_0", "value")).ok();
        let result: String = client.execute(client.select(1)).unwrap();
        assert_eq!(result.to_lowercase(), "ok");

        let dbsize: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(dbsize, 0);

        client.execute::<()>(client.select(0)).ok();
        let value: Option<String> = client.execute(client.get("key_in_0")).unwrap();
        assert_eq!(value, Some("value".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SCAN — Incremental iteration
#[test]
fn test_integration_admin_scan() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..50 {
            client
                .execute::<()>(client.set(format!("scan_key_{i}"), format!("val_{i}")))
                .ok();
        }

        let result: (i64, Vec<String>) = client.execute(client.scan(0)).unwrap();
        let (cursor, keys) = result;
        assert!(cursor >= 0);
        assert!(!keys.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}
