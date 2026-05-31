#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::{AdminCommands, SetsCommands, StringsCommands};

// SCARD — Get set cardinality
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_card() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Non-existent set has 0 members
        let scard: i64 = client.execute(client.scard("nonexistent")).unwrap();
        assert_eq!(scard, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SMEMBERS — Get all members
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_members() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let members: Vec<String> =
            client.execute(client.smembers("nonexistent")).unwrap();
        assert!(members.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SPOP — Pop random member
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_pop() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Non-existent set returns None
        let popped: Option<String> =
            client.execute(client.spop("nonexistent")).unwrap();
        assert!(popped.is_none());

        let popped_multi: Vec<String> =
            client.execute(client.spop_count("nonexistent", 1)).unwrap();
        assert!(popped_multi.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SRANDMEMBER — Random member
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_randmember() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Non-existent set returns None
        let rand_single: Option<String> =
            client.execute(client.srandmember("nonexistent")).unwrap();
        assert!(rand_single.is_none());

        let rand_multi: Vec<String> = client
            .execute(client.srandmember_count("nonexistent", 1))
            .unwrap();
        assert!(rand_multi.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SINTER — Intersection
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_intersect() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let intersection: Vec<String> =
            client.execute(client.sinter(&["setA", "setB"])).unwrap();
        assert!(intersection.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SUNION — Union
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_union() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let union: Vec<String> =
            client.execute(client.sunion(&["setA", "setB"])).unwrap();
        assert!(union.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// SSCAN — Scan set members
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_scan() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: (i64, Vec<String>) =
            client.execute(client.sscan("nonexistent", 0)).unwrap();
        let (cursor, members) = result;
        assert!(cursor >= 0);
        assert!(members.is_empty());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// Non-existent set returns empty for all operations
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_nonexistent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let members: Vec<String> =
            client.execute(client.smembers("nonexistent")).unwrap();
        assert!(members.is_empty());

        let scard: i64 = client.execute(client.scard("nonexistent")).unwrap();
        assert_eq!(scard, 0);

        let popped: Option<String> =
            client.execute(client.spop("nonexistent")).unwrap();
        assert!(popped.is_none());

        client.execute::<()>(client.flushdb()).ok();
    });
}

// Concurrent set operations
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_sets_concurrent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute::<()>(c1.set("set_concurrent:a", "val1")).ok();
        c2.execute::<()>(c2.set("set_concurrent:b", "val2")).ok();

        let type_a: String = client.execute(client.type_("set_concurrent:a")).unwrap();
        assert_eq!(type_a.to_lowercase(), "string");

        let type_b: String = client.execute(client.type_("set_concurrent:b")).unwrap();
        assert_eq!(type_b.to_lowercase(), "string");

        client.execute::<()>(client.flushdb()).ok();
    });
}
