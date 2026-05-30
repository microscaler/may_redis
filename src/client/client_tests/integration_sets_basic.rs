#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::SetsCommands;

// ---------------------------------------------------------------------------
// SADD — Add members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_sadd() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let added: i64 = client.execute(client.sadd("set1", "alice")).unwrap();
        assert_eq!(added, 1);

        let added: i64 = client.execute(client.sadd("set1", "bob")).unwrap();
        assert_eq!(added, 1);

        let added: i64 = client.execute(client.sadd("set1", "charlie")).unwrap();
        assert_eq!(added, 1);

        // Adding duplicate returns 0
        let added: i64 = client.execute(client.sadd("set1", "alice")).unwrap();
        assert_eq!(added, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SMEMBERS — Get all members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_smembers() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("set2", "x")).ok();
        client.execute(client.sadd("set2", "y")).ok();
        client.execute(client.sadd("set2", "z")).ok();

        let members: Vec<String> = client.execute(client.smembers("set2")).unwrap();
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"x".to_string()));
        assert!(members.contains(&"y".to_string()));
        assert!(members.contains(&"z".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SISMEMBER — Check membership
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_sismember() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("set3", "present")).ok();

        let present: i64 = client.execute(client.sismember("set3", "present")).unwrap();
        assert_eq!(present, 1);

        let absent: i64 = client.execute(client.sismember("set3", "absent")).unwrap();
        assert_eq!(absent, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SREM — Remove members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_srem() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("set4", "a")).ok();
        client.execute(client.sadd("set4", "b")).ok();
        client.execute(client.sadd("set4", "c")).ok();

        let removed: i64 = client.execute(client.srem("set4", "b")).unwrap();
        assert_eq!(removed, 1);

        let members: Vec<String> = client.execute(client.smembers("set4")).unwrap();
        assert_eq!(members.len(), 2);
        assert!(!members.contains(&"b".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SCARD — Count members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_card() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let empty: i64 = client.execute(client.scard("empty_set")).unwrap();
        assert_eq!(empty, 0);

        client.execute(client.sadd("set5", "one")).ok();
        client.execute(client.sadd("set5", "two")).ok();

        let count: i64 = client.execute(client.scard("set5")).unwrap();
        assert_eq!(count, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SPOP — Pop random members
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_spop() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("set6", "first")).ok();
        client.execute(client.sadd("set6", "second")).ok();
        client.execute(client.sadd("set6", "third")).ok();

        let popped: Option<String> = client.execute(client.spop("set6")).unwrap();
        assert!(popped.is_some());

        let members: Vec<String> = client.execute(client.smembers("set6")).unwrap();
        assert_eq!(members.len(), 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SRANDMEMBER — Random member without pop
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_srandmember() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("set7", "alpha")).ok();
        client.execute(client.sadd("set7", "beta")).ok();

        let rand_single: Option<String> = client.execute(client.srandmember("set7")).unwrap();
        assert!(rand_single.is_some());

        let rand_multi: Vec<String> = client.execute(client.srandmember_n("set7", 1)).unwrap();
        assert!(!rand_multi.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SINTER — Set intersection
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_sinter() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("setA", "common")).ok();
        client.execute(client.sadd("setA", "only_a")).ok();
        client.execute(client.sadd("setB", "common")).ok();
        client.execute(client.sadd("setB", "only_b")).ok();

        let intersection: Vec<String> = client.execute(client.sinter("setA", "setB")).unwrap();
        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&"common".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SUNION — Set union
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_sunion() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute(client.sadd("setC", "one")).ok();
        client.execute(client.sadd("setC", "two")).ok();
        client.execute(client.sadd("setD", "two")).ok();
        client.execute(client.sadd("setD", "three")).ok();

        let union: Vec<String> = client.execute(client.sunion("setC", "setD")).unwrap();
        assert_eq!(union.len(), 3);
        assert!(union.contains(&"one".to_string()));
        assert!(union.contains(&"two".to_string()));
        assert!(union.contains(&"three".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SSCAN — Incremental iteration
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_sscan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..100 {
            client.execute(client.sadd("set8", format!("member_{i}"))).ok();
        }

        let result: (i64, Vec<String>) = client.execute(client.sscan("set8", 0)).unwrap();
        let (cursor, members) = result;
        assert!(cursor >= 0);
        assert!(!members.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Set on non-existent key
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_empty_key() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let members: Vec<String> = client.execute(client.smembers("nonexistent")).unwrap();
        assert!(members.is_empty());

        let sismember: i64 = client.execute(client.sismember("nonexistent", "x")).unwrap();
        assert_eq!(sismember, 0);

        let scard: i64 = client.execute(client.scard("nonexistent")).unwrap();
        assert_eq!(scard, 0);

        let srem: i64 = client.execute(client.srem("nonexistent", "x")).unwrap();
        assert_eq!(srem, 0);

        let spop: Option<String> = client.execute(client.spop("nonexistent")).unwrap();
        assert!(spop.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}
