#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::ListsCommands;

// ---------------------------------------------------------------------------
// LPUSH / RPUSH — Push to front and back
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lpush_rpush() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // LPUSH prepends
        client.execute(client.lpush("list1", "third")).ok();
        client.execute(client.lpush("list1", "second")).ok();
        client.execute(client.lpush("list1", "first")).ok();

        // RPUSH appends
        client.execute(client.rpush("list2", "first")).ok();
        client.execute(client.rpush("list2", "second")).ok();

        // Verify order via LRANGE
        let range: Vec<String> = client.execute(client.lrange("list1", 0, -1)).unwrap();
        assert_eq!(range, vec!["first", "second", "third"]);

        let range: Vec<String> = client.execute(client.lrange("list2", 0, -1)).unwrap();
        assert_eq!(range, vec!["first", "second"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LPOP / RPOP — Pop from front and back
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lpop_rpop() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.rpush("list3", "a")).ok();
        client.execute(client.rpush("list3", "b")).ok();
        client.execute(client.rpush("list3", "c")).ok();

        // LPOP removes from front
        let popped: Option<String> = client.execute(client.lpop("list3")).unwrap();
        assert_eq!(popped, Some("a".to_string()));

        // RPOP removes from back
        let popped: Option<String> = client.execute(client.rpop("list3")).unwrap();
        assert_eq!(popped, Some("c".to_string()));

        // Only "b" remains
        let len: i64 = client.execute(client.llen("list3")).unwrap();
        assert_eq!(len, 1);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LRANGE — Get range of elements
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lrange() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..10 {
            client.execute(client.rpush("list4", format!("item_{i}"))).ok();
        }

        // Get first 3
        let range: Vec<String> = client.execute(client.lrange("list4", 0, 2)).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0], "item_0");
        assert_eq!(range[2], "item_2");

        // Get last 3
        let range: Vec<String> = client.execute(client.lrange("list4", -3, -1)).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[2], "item_9");

        // Get all
        let all: Vec<String> = client.execute(client.lrange("list4", 0, -1)).unwrap();
        assert_eq!(all.len(), 10);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LINDEX — Get element by index
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lindex() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.rpush("list5", "alpha")).ok();
        client.execute(client.rpush("list5", "beta")).ok();
        client.execute(client.rpush("list5", "gamma")).ok();

        let idx: Option<String> = client.execute(client.lindex("list5", 0)).unwrap();
        assert_eq!(idx, Some("alpha".to_string()));

        let idx: Option<String> = client.execute(client.lindex("list5", 1)).unwrap();
        assert_eq!(idx, Some("beta".to_string()));

        let idx: Option<String> = client.execute(client.lindex("list5", -1)).unwrap();
        assert_eq!(idx, Some("gamma".to_string()));

        let idx: Option<String> = client.execute(client.lindex("list5", 99)).unwrap();
        assert!(idx.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LLEN — Get list length
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_llen() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let len: i64 = client.execute(client.llen("empty_list")).unwrap();
        assert_eq!(len, 0);

        client.execute(client.rpush("list6", "x")).ok();
        client.execute(client.rpush("list6", "y")).ok();
        client.execute(client.rpush("list6", "z")).ok();

        let len: i64 = client.execute(client.llen("list6")).unwrap();
        assert_eq!(len, 3);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LSET — Set element by index
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lset() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.rpush("list7", "old_a")).ok();
        client.execute(client.rpush("list7", "old_b")).ok();

        client.execute(client.lset("list7", 0, "new_a")).ok();
        client.execute(client.lset("list7", 1, "new_b")).ok();

        let range: Vec<String> = client.execute(client.lrange("list7", 0, -1)).unwrap();
        assert_eq!(range, vec!["new_a", "new_b"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LREM — Remove by value
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_lrem() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.rpush("list8", "a")).ok();
        client.execute(client.rpush("list8", "b")).ok();
        client.execute(client.rpush("list8", "a")).ok();
        client.execute(client.rpush("list8", "b")).ok();
        client.execute(client.rpush("list8", "a")).ok();

        // Remove all "a" (count=0)
        let removed: i64 = client.execute(client.lrem("list8", "a")).unwrap();
        assert_eq!(removed, 3);

        let range: Vec<String> = client.execute(client.lrange("list8", 0, -1)).unwrap();
        assert_eq!(range, vec!["b", "b"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// LTRIM — Trim list to range
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_ltrim() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..10 {
            client.execute(client.rpush("list9", format!("item_{i}"))).ok();
        }

        // Keep only items 3 through 6 (inclusive)
        client.execute(client.ltrim("list9", 3, 6)).ok();

        let range: Vec<String> = client.execute(client.lrange("list9", 0, -1)).unwrap();
        assert_eq!(range.len(), 4);
        assert_eq!(range, vec!["item_3", "item_4", "item_5", "item_6"]);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// List on non-existent key
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_empty_key() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let len: i64 = client.execute(client.llen("nonexistent")).unwrap();
        assert_eq!(len, 0);

        let range: Vec<String> = client.execute(client.lrange("nonexistent", 0, -1)).unwrap();
        assert!(range.is_empty());

        let lpop: Option<String> = client.execute(client.lpop("nonexistent")).unwrap();
        assert!(lpop.is_none());

        let rpop: Option<String> = client.execute(client.rpop("nonexistent")).unwrap();
        assert!(rpop.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// List — concurrent access
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_list_concurrent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute(client.lpush("list_concurrent", "from_c1")).ok();
        c2.execute(client.lpush("list_concurrent", "from_c2")).ok();

        let len: i64 = client.execute(client.llen("list_concurrent")).unwrap();
        assert_eq!(len, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}
