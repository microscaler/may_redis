#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::used_underscore_items
)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, StringsCommands};

// ---------------------------------------------------------------------------
// INCRBY — Increment by a value
// ---------------------------------------------------------------------------

#[test]
fn test_strings_incrby_basic() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("counter", "0")).ok();
        let val: i64 = client.execute(client.incrby("counter", 5)).unwrap();
        assert_eq!(val, 5);

        let val: i64 = client.execute(client.incrby("counter", -2)).unwrap();
        assert_eq!(val, 3);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_strings_incrby_overflow() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("big", "9223372036854775806"))
            .ok();
        let err = client
            .execute::<i64>(client.incrby("big", 100))
            .unwrap_err();
        assert!(err.to_string().contains("overflows"), "error: {err:?}");

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// DECR / DECRBY — Decrement operations
// ---------------------------------------------------------------------------

#[test]
fn test_strings_decr_basic() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("dec", "50")).ok();
        let val: i64 = client.execute(client.decr("dec")).unwrap();
        assert_eq!(val, 49);

        let val: i64 = client.execute(client.decrby("dec", 10)).unwrap();
        assert_eq!(val, 39);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_strings_decr_negative() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("neg", "-5")).ok();
        let val: i64 = client.execute(client.decr("neg")).unwrap();
        assert_eq!(val, -6);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SETNX — Set if not exists
// ---------------------------------------------------------------------------

#[test]
fn test_strings_setnx() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let exists: i64 = client.execute(client.setnx("new_key", "hello")).unwrap();
        assert_eq!(exists, 1);

        let val: Option<String> = client.execute(client.get("new_key")).unwrap();
        assert_eq!(val, Some("hello".to_string()));

        // Updating existing key via SETNX returns 0
        let exists: i64 = client.execute(client.setnx("new_key", "world")).unwrap();
        assert_eq!(exists, 0);

        let val: Option<String> = client.execute(client.get("new_key")).unwrap();
        assert_eq!(val, Some("hello".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// APPEND — Append to a string
// ---------------------------------------------------------------------------

#[test]
fn test_strings_append() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("msg", "hello")).ok();
        let len: i64 = client.execute(client.append("msg", ", world")).unwrap();
        assert_eq!(len, 12);

        let val: Option<String> = client.execute(client.get("msg")).unwrap();
        assert_eq!(val, Some("hello, world".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_strings_append_nonexistent() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let len: i64 = client.execute(client.append("new", "initial")).unwrap();
        assert_eq!(len, 7);

        let val: Option<String> = client.execute(client.get("new")).unwrap();
        assert_eq!(val, Some("initial".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// MGET — Get multiple keys
// ---------------------------------------------------------------------------

#[test]
fn test_strings_mget() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("a", "1")).ok();
        client.execute::<()>(client.set("b", "2")).ok();
        client.execute::<()>(client.set("c", "3")).ok();

        let vals: Vec<Option<String>> = client
            .execute(client.mget(&["a", "b", "c", "missing"]))
            .unwrap();
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0], Some("1".to_string()));
        assert_eq!(vals[1], Some("2".to_string()));
        assert_eq!(vals[2], Some("3".to_string()));
        assert_eq!(vals[3], None);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
fn test_strings_mget_empty() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let vals: Vec<Option<String>> =
            client.execute(client.mget(&["x", "y"])).unwrap();
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0], None);
        assert_eq!(vals[1], None);

        client.execute::<()>(client.flushdb()).ok();
    });
}
