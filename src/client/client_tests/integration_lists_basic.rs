#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, ListsCommands};

// LPOP / RPOP — Pop from list
#[test]
fn test_integration_lists_pop() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // On non-existent key, lpop/rpop return None
        let lpop: Option<String> = client.execute(client.lpop("nonexistent")).unwrap();
        assert!(lpop.is_none());

        let rpop: Option<String> = client.execute(client.rpop("nonexistent")).unwrap();
        assert!(rpop.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// LLEN — Get list length
#[test]
fn test_integration_lists_len() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Non-existent list has length 0
        let len: i64 = client.execute(client.llen("nonexistent")).unwrap();
        assert_eq!(len, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// LRANGE — Get range of elements
#[test]
fn test_integration_lists_range() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Non-existent list returns empty range
        let range: Vec<String> =
            client.execute(client.lrange("nonexistent", 0, -1)).unwrap();
        assert!(range.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// LINDEX — Get element by index
#[test]
fn test_integration_lists_index() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let index: Option<String> =
            client.execute(client.lindex("nonexistent", 0)).unwrap();
        assert!(index.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// LTRIM — Trim list to range
#[test]
fn test_integration_lists_trim() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // LTRIM on non-existent key does nothing
        client.execute::<()>(client.ltrim("nonexistent", 0, 5)).ok();

        client.execute::<()>(client.flushdb()).ok();
    });
}
