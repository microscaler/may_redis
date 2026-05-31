#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::{AdminCommands, SortedSetsCommands};

// ZADD — Add members with scores
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_add() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Add member (returns whether new member added)
        let added: i64 = client.execute(client.zadd("zset1", 87.3, "bob")).unwrap();
        assert_eq!(added, 1);

        // Updating score returns 0 (no new member added)
        let added: i64 = client.execute(client.zadd("zset1", 90.0, "bob")).unwrap();
        assert_eq!(added, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZREM — Remove members
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_remove() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Remove from non-existent set
        let removed: i64 = client
            .execute(client.zrem("nonexistent", "member"))
            .unwrap();
        assert_eq!(removed, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZCARD — Get set cardinality
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_card() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let zcard: i64 = client.execute(client.zcard("nonexistent")).unwrap();
        assert_eq!(zcard, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZRANK — Get rank of member
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_rank() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let rank: Option<i64> = client
            .execute(client.zrank("nonexistent", "missing"))
            .unwrap();
        assert!(rank.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZSCORE — Get score of member
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_score() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let score: Option<f64> =
            client.execute(client.zscore("nonexistent", "x")).unwrap();
        assert!(score.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZCOUNT — Count members in score range
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_count() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let count: i64 = client
            .execute(client.zcount("nonexistent", 0.0, f64::INFINITY))
            .unwrap();
        assert_eq!(count, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZPOPMAX — Pop member with highest score
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_popmax() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: Vec<(String, f64)> = client
            .execute(client.zpopmax_count("nonexistent", 1))
            .unwrap();
        assert!(result.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZPOPMIN — Pop member with lowest score
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_popmin() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: Vec<(String, f64)> = client
            .execute(client.zpopmin_count("nonexistent", 1))
            .unwrap();
        assert!(result.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZRANGE — Get range by index
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_range() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let range: Vec<String> =
            client.execute(client.zrange("nonexistent", 0, -1)).unwrap();
        assert!(range.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ZSCAN — Scan sorted set
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_scan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: (i64, Vec<(String, f64)>) =
            client.execute(client.zscan("nonexistent", 0)).unwrap();
        let (cursor, members) = result;
        assert!(cursor >= 0);
        assert!(members.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// Non-existent set returns empty for all operations
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_sets_nonexistent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let zcard: i64 = client.execute(client.zcard("nonexistent")).unwrap();
        assert_eq!(zcard, 0);

        let zrank: Option<i64> =
            client.execute(client.zrank("nonexistent", "x")).unwrap();
        assert!(zrank.is_none());

        let zscore: Option<f64> =
            client.execute(client.zscore("nonexistent", "x")).unwrap();
        assert!(zscore.is_none());

        let zrem: i64 = client.execute(client.zrem("nonexistent", "x")).unwrap();
        assert_eq!(zrem, 0);

        let range: Vec<String> =
            client.execute(client.zrange("nonexistent", 0, -1)).unwrap();
        assert!(range.is_empty());

        let count: i64 = client
            .execute(client.zcount("nonexistent", 0.0, f64::INFINITY))
            .unwrap();
        assert_eq!(count, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}
