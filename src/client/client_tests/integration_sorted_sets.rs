#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::SortedSetsCommands;

// ---------------------------------------------------------------------------
// ZADD — Add members with scores
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zadd() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let added: i64 = client.execute(client.zadd("zset1", "alice", 95.5)).unwrap();
        assert_eq!(added, 1);

        let added: i64 = client.execute(client.zadd("zset1", "bob", 87.3)).unwrap();
        assert_eq!(added, 1);

        let added: i64 = client.execute(client.zadd("zset1", "charlie", 92.0)).unwrap();
        assert_eq!(added, 1);

        // Updating score returns 0 (no new member added)
        let added: i64 = client.execute(client.zadd("zset1", "alice", 96.0)).unwrap();
        assert_eq!(added, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZREM — Remove members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zrem() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset2", "a", 1.0)).ok();
        client.execute(client.zadd("zset2", "b", 2.0)).ok();
        client.execute(client.zadd("zset2", "c", 3.0)).ok();

        let removed: i64 = client.execute(client.zrem("zset2", "b")).unwrap();
        assert_eq!(removed, 1);

        let len: i64 = client.execute(client.zcard("zset2")).unwrap();
        assert_eq!(len, 2);

        // Remove non-existent member returns 0
        let removed: i64 = client.execute(client.zrem("zset2", "nonexistent")).unwrap();
        assert_eq!(removed, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZCARD — Count members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zcard() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let empty: i64 = client.execute(client.zcard("empty_zset")).unwrap();
        assert_eq!(empty, 0);

        client.execute(client.zadd("zset3", "x", 10.0)).ok();
        client.execute(client.zadd("zset3", "y", 20.0)).ok();

        let count: i64 = client.execute(client.zcard("zset3")).unwrap();
        assert_eq!(count, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZRANK — Get rank of member
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zrank() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset4", "first", 1.0)).ok();
        client.execute(client.zadd("zset4", "second", 2.0)).ok();
        client.execute(client.zadd("zset4", "third", 3.0)).ok();

        // ZRANK returns 0-based rank by score (ascending)
        let rank: Option<i64> = client.execute(client.zrank("zset4", "first")).unwrap();
        assert_eq!(rank, Some(0));

        let rank: Option<i64> = client.execute(client.zrank("zset4", "third")).unwrap();
        assert_eq!(rank, Some(2));

        // Non-existent member returns None
        let rank: Option<i64> = client.execute(client.zrank("zset4", "missing")).unwrap();
        assert!(rank.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZSCORE — Get score of member
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zscore() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset5", "player1", 42.5)).ok();

        let score: Option<f64> = client.execute(client.zscore("zset5", "player1")).unwrap();
        assert_eq!(score, Some(42.5));

        let score: Option<f64> = client.execute(client.zscore("zset5", "missing")).unwrap();
        assert!(score.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZCOUNT — Count members in score range
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zcount() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset6", "a", 1.0)).ok();
        client.execute(client.zadd("zset6", "b", 5.0)).ok();
        client.execute(client.zadd("zset6", "c", 10.0)).ok();
        client.execute(client.zadd("zset6", "d", 15.0)).ok();

        // Count members with score between 1 and 10 (inclusive)
        let count: i64 = client.execute(client.zcount("zset6", 1.0, 10.0)).unwrap();
        assert_eq!(count, 3);

        // Count members with score > 10
        let count: i64 = client.execute(client.zcount("zset6", 10.1, f64::INFINITY)).unwrap();
        assert_eq!(count, 1);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZINCRBY — Increment member score
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zincrby() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset7", "counter", 10.0)).ok();

        let new_score: f64 = client.execute(client.zincrby("zset7", 5.0, "counter")).unwrap();
        assert_eq!(new_score, 15.0);

        let new_score: f64 = client.execute(client.zincrby("zset7", -3.0, "counter")).unwrap();
        assert_eq!(new_score, 12.0);

        let score: Option<f64> = client.execute(client.zscore("zset7", "counter")).unwrap();
        assert_eq!(score, Some(12.0));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZPOPMAX / ZPOPMIN — Pop highest/lowest score members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_pop() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset8", "low", 1.0)).ok();
        client.execute(client.zadd("zset8", "mid", 5.0)).ok();
        client.execute(client.zadd("zset8", "high", 10.0)).ok();

        // ZPOPMAX returns member with highest score
        let result: Vec<(String, f64)> = client.execute(client.zpopmax_n("zset8", 1)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "high");

        // ZPOPMIN returns member with lowest score
        let result: Vec<(String, f64)> = client.execute(client.zpopmin_n("zset8", 1)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "low");

        let remaining: i64 = client.execute(client.zcard("zset8")).unwrap();
        assert_eq!(remaining, 1);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZRANGE — Get range of members by rank
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zrange() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset9", "first", 1.0)).ok();
        client.execute(client.zadd("zset9", "second", 2.0)).ok();
        client.execute(client.zadd("zset9", "third", 3.0)).ok();
        client.execute(client.zadd("zset9", "fourth", 4.0)).ok();

        let range: Vec<String> = client.execute(client.zrange("zset9", 0, 1)).unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0], "first");
        assert_eq!(range[1], "second");

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZRANGEBYSCORE — Get members in score range
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zrangebyscore() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.zadd("zset10", "a", 1.0)).ok();
        client.execute(client.zadd("zset10", "b", 3.0)).ok();
        client.execute(client.zadd("zset10", "c", 5.0)).ok();
        client.execute(client.zadd("zset10", "d", 7.0)).ok();

        let range: Vec<String> = client.execute(client.zrangebyscore("zset10", 2.0, 6.0)).unwrap();
        assert_eq!(range.len(), 2);
        assert!(range.contains(&"b".to_string()));
        assert!(range.contains(&"c".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// ZSCAN — Incremental iteration
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_zscan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..100 {
            client.execute(client.zadd("zset11", format!("member_{i}"), i as f64)).ok();
        }

        let result: (i64, Vec<(String, f64)>) = client.execute(client.zscan("zset11", 0)).unwrap();
        let (cursor, members) = result;
        assert!(cursor >= 0);
        assert!(!members.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SortedSet on non-existent key
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sorted_set_empty_key() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let zcard: i64 = client.execute(client.zcard("nonexistent")).unwrap();
        assert_eq!(zcard, 0);

        let zrank: Option<i64> = client.execute(client.zrank("nonexistent", "x")).unwrap();
        assert!(zrank.is_none());

        let zscore: Option<f64> = client.execute(client.zscore("nonexistent", "x")).unwrap();
        assert!(zscore.is_none());

        let zrem: i64 = client.execute(client.zrem("nonexistent", "x")).unwrap();
        assert_eq!(zrem, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}
