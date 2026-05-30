#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::{AdminCommands, PubsubCommands, StringsCommands};

#[test]
fn test_command_get_encoding() {
    let buf = ().get("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_set_encoding() {
    let buf = ().set("key", "val").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n"
    );
}
#[test]
fn test_command_set_ex_encoding() {
    let buf = ().set_ex("key", "val", 60).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n$2\r\nEX\r\n$2\r\n60\r\n"
    );
}
#[test]
fn test_command_exists_encoding() {
    let buf = ().exists("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$6\r\nEXISTS\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_del_encoding() {
    let buf = ().del("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nDEL\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_incr_encoding() {
    let buf = ().incr("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINCR\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_ttl_encoding() {
    let buf = ().ttl("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nTTL\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_expire_encoding() {
    let buf = ().expire("key", 60).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nEXPIRE\r\n$3\r\nkey\r\n$2\r\n60\r\n"
    );
}
#[test]
fn test_command_publish_encoding() {
    let buf = ().publish("channel", "message").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$7\r\nPUBLISH\r\n$7\r\nchannel\r\n$7\r\nmessage\r\n"
    );
}
#[test]
fn test_command_keys_encoding() {
    let buf = ().keys("*").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n");
}
#[test]
fn test_command_dbsize_encoding() {
    let buf = ().dbsize().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$6\r\nDBSIZE\r\n");
}
#[test]
fn test_command_flushdb_encoding() {
    let buf = ().flushdb().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$7\r\nFLUSHDB\r\n");
}
#[test]
fn test_command_ping_encoding() {
    let buf = ().ping().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nPING\r\n");
}
#[test]
fn test_command_auth_encoding() {
    let buf = ().auth("secret").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nAUTH\r\n$6\r\nsecret\r\n");
}
#[test]
fn test_command_setex_encoding() {
    let buf = ().setex("key", 60, "val").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nSETEX\r\n$3\r\nkey\r\n$2\r\n60\r\n$3\r\nval\r\n"
    );
}
#[test]
fn test_command_incrby_encoding() {
    let buf = ().incrby("counter", 5).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nINCRBY\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
    );
}
#[test]
fn test_command_append_encoding() {
    let buf = ().append("key", "hello").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nAPPEND\r\n$3\r\nkey\r\n$5\r\nhello\r\n"
    );
}
#[test]
fn test_command_decr_encoding() {
    let buf = ().decr("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nDECR\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_decrby_encoding() {
    let buf = ().decrby("counter", 5).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nDECRBY\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
    );
}
#[test]
fn test_command_setnx_encoding() {
    let buf = ().setnx("key", "value").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nSETNX\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
    );
}
#[test]
fn test_command_mget_encoding() {
    let buf = <() as StringsCommands>::mget(&(), &["key1", "key2"])
        .build()
        .unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nMGET\r\n$4\r\nkey1\r\n$4\r\nkey2\r\n"
    );
}
#[test]
fn test_command_mset_encoding() {
    let buf = <() as StringsCommands>::mset(&(), &[("key1", "val1"), ("key2", "val2")])
        .build()
        .unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$4\r\nMSET\r\n$4\r\nkey1\r\n$4\r\nval1\r\n$4\r\nkey2\r\n$4\r\nval2\r\n"
    );
}
#[test]
fn test_command_msetnx_encoding() {
    let buf = <() as StringsCommands>::msetnx(&(), &[("key1", "val1"), ("key2", "val2")])
        .build()
        .unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$6\r\nMSETNX\r\n$4\r\nkey1\r\n$4\r\nval1\r\n$4\r\nkey2\r\n$4\r\nval2\r\n"
    );
}
#[test]
fn test_command_strlen_encoding() {
    let buf = ().strlen("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$6\r\nSTRLEN\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_getrange_encoding() {
    let buf = ().getrange("key", 0, -1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$8\r\nGETRANGE\r\n$3\r\nkey\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
}
#[test]
fn test_command_setrange_encoding() {
    let buf = ().setrange("key", 5, "value").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$8\r\nSETRANGE\r\n$3\r\nkey\r\n$1\r\n5\r\n$5\r\nvalue\r\n"
    );
}
#[test]
fn test_command_setbit_encoding() {
    let buf = ().setbit("key", 0, 1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$6\r\nSETBIT\r\n$3\r\nkey\r\n$1\r\n0\r\n$1\r\n1\r\n"
    );
}
#[test]
fn test_command_getbit_encoding() {
    let buf = ().getbit("key", 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nGETBIT\r\n$3\r\nkey\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_bitcount_encoding() {
    let buf = ().bitcount("key").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$8\r\nBITCOUNT\r\n$3\r\nkey\r\n");
}
#[test]
fn test_command_bitcount_range_encoding() {
    let buf = ().bitcount_range("key", 0, -1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$8\r\nBITCOUNT\r\n$3\r\nkey\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
}
