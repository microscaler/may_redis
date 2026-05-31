#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::used_underscore_items
)]

use super::unit::{run_may, shared_client};
use crate::protocol::commands::{AdminCommands, HashesCommands, StringsCommands};

// ---------------------------------------------------------------------------
// MSET — Set multiple keys at once
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_mset() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let pairs = [("x", "10"), ("y", "20"), ("z", "30")];
        client.execute::<()>(client.mset(&pairs)).ok();

        let val: Option<String> = client.execute(client.get("x")).unwrap();
        assert_eq!(val, Some("10".to_string()));
        let val: Option<String> = client.execute(client.get("y")).unwrap();
        assert_eq!(val, Some("20".to_string()));
        let val: Option<String> = client.execute(client.get("z")).unwrap();
        assert_eq!(val, Some("30".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// MSETNX — Set multiple keys only if none exist
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_msetnx_all_new() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let pairs = [("m1", "a"), ("m2", "b")];
        let result: i64 = client.execute(client.msetnx(&pairs)).unwrap();
        assert_eq!(result, 1);

        let val: Option<String> = client.execute(client.get("m1")).unwrap();
        assert_eq!(val, Some("a".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_msetnx_existing() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("m1", "original")).ok();

        let pairs = [("m1", "new_a"), ("m2", "new_b")];
        let result: i64 = client.execute(client.msetnx(&pairs)).unwrap();
        assert_eq!(result, 0);

        // Original values should be unchanged
        let val: Option<String> = client.execute(client.get("m1")).unwrap();
        assert_eq!(val, Some("original".to_string()));
        let val: Option<String> = client.execute(client.get("m2")).unwrap();
        assert_eq!(val, None);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// STRLEN — Get string length
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_strlen() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("text", "hello")).ok();
        let len: i64 = client.execute(client.strlen("text")).unwrap();
        assert_eq!(len, 5);

        let len: i64 = client.execute(client.strlen("missing")).unwrap();
        assert_eq!(len, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// GETRANGE — Get substring
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_getrange() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("greeting", "Hello, World!"))
            .ok();

        // [0:4] => "Hello"
        let val: Option<String> =
            client.execute(client.getrange("greeting", 0, 4)).unwrap();
        assert_eq!(val, Some("Hello".to_string()));

        // [7:12] => "World!"
        let val: Option<String> =
            client.execute(client.getrange("greeting", 7, 12)).unwrap();
        assert_eq!(val, Some("World!".to_string()));

        // [-3:] last 3 chars
        let val: Option<String> =
            client.execute(client.getrange("greeting", -3, -1)).unwrap();
        assert_eq!(val, Some("rld".to_string()));

        // Out-of-range end
        let val: Option<String> =
            client.execute(client.getrange("greeting", 0, 999)).unwrap();
        assert_eq!(val, Some("Hello, World!".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SETRANGE — Overwrite part of a string
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_setrange() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("orig", "Hello World!"))
            .ok();
        let len: i64 = client.execute(client.setrange("orig", 6, "Rust")).unwrap();
        assert_eq!(len, 12);

        let val: Option<String> = client.execute(client.get("orig")).unwrap();
        assert_eq!(val, Some("Hello Rust!".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_setrange_offset_beyond() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("s", "ab")).ok();
        let len: i64 = client.execute(client.setrange("s", 10, "cd")).unwrap();
        assert_eq!(len, 12);

        let val: Option<String> = client.execute(client.get("s")).unwrap();
        assert!(val.is_some());
        // "ab\0\0\0\0\0\0cd" — 12 bytes with null padding

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SETBIT / GETBIT / BITCOUNT — Bit-level operations
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_setbit_getbit() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Set bit at offset 7 to 1 → "10000000" in big-endian
        let prev: i64 = client.execute(client.setbit("bits", 0, 1)).unwrap();
        assert_eq!(prev, 0);

        let bit: i64 = client.execute(client.getbit("bits", 0)).unwrap();
        assert_eq!(bit, 1);

        let bit: i64 = client.execute(client.getbit("bits", 7)).unwrap();
        assert_eq!(bit, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_bitcount() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Set 0xFF → "11111111" → 8 bits set
        client
            .execute::<()>(client.set("full", String::from_utf8(vec![0xff]).unwrap()))
            .ok();
        let count: i64 = client.execute(client.bitcount("full")).unwrap();
        assert_eq!(count, 8);

        // Set 0x55 → "01010101" → 4 bits set
        client.execute::<()>(client.set("half", "U")).ok();
        let count: i64 = client.execute(client.bitcount("half")).unwrap();
        assert_eq!(count, 4);

        // Byte range: just the first byte
        client
            .execute::<()>(client.set("full", String::from_utf8(vec![0xff]).unwrap()))
            .ok();
        let count: i64 = client.execute(client.bitcount_range("full", 0, 0)).unwrap();
        assert_eq!(count, 8);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// SETEX — Set with expiry
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_setex() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.setex("ttl", 1, "expire_me"))
            .ok();
        let val: Option<String> = client.execute(client.get("ttl")).unwrap();
        assert_eq!(val, Some("expire_me".to_string()));

        let pttl: i64 = client.execute(client.pttl("ttl")).unwrap();
        assert!(pttl > 0 && pttl <= 1000);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_setex_large_ttl() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.setex("long", 86400, "day"))
            .ok();
        let val: Option<String> = client.execute(client.get("long")).unwrap();
        assert_eq!(val, Some("day".to_string()));

        let ttl: i64 = client.execute(client.ttl("long")).unwrap();
        assert!(ttl > 80000);

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// Type mismatch error — string command on non-string key
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_incr_wrong_type() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Put a hash key, then try INCR
        client.execute::<()>(client.hset("hashkey", "f", "v")).ok();
        let err = client.execute::<i64>(client.incr("hashkey")).unwrap_err();
        assert!(
            err.to_string().contains("WRONGTYPE"),
            "error should be WRONGTYPE, got: {err:?}"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_strings_append_wrong_type() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.hset("hashkey2", "f", "v")).ok();
        let err = client
            .execute::<i64>(client.append("hashkey2", "data"))
            .unwrap_err();
        assert!(
            err.to_string().contains("WRONGTYPE"),
            "error should be WRONGTYPE, got: {err:?}"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}
