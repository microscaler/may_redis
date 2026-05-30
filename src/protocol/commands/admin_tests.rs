#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::builder::CommandPolicy;

/// Helper for encoding tests of commands that the default CommandPolicy
/// blocks (FLUSHALL, KEYS, CONFIG, SCAN variants, SHUTDOWN, etc.).
/// These tests verify wire format, not policy enforcement.
fn blocked_cmd(args: &[&str]) -> Option<bytes::BytesMut> {
    let mut builder = crate::protocol::builder::CommandBuilder::new(args[0]);
    for arg in &args[1..] {
        builder = builder.arg(*arg);
    }
    builder.build_with_policy(CommandPolicy::PERMISSIVE)
}

#[test]
fn test_command_scan_encoding() {
    let buf = blocked_cmd(&["SCAN", "0"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nSCAN\r\n$1\r\n0\r\n");
}
#[test]
fn test_command_scan_match_encoding() {
    let buf = blocked_cmd(&["SCAN", "0", "MATCH", "foo*"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nSCAN\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$4\r\nfoo*\r\n"
    );
}
#[test]
fn test_command_touch_encoding() {
    let buf = blocked_cmd(&["TOUCH", "k1", "k2", "k3"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nTOUCH\r\n$2\r\nk1\r\n$2\r\nk2\r\n$2\r\nk3\r\n"
    );
}
#[test]
fn test_command_save_encoding() {
    let buf = blocked_cmd(&["SAVE"]).unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nSAVE\r\n");
}
#[test]
fn test_command_bgsave_encoding() {
    let buf = blocked_cmd(&["BGSAVE"]).unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$6\r\nBGSAVE\r\n");
}
#[test]
fn test_command_flushall_encoding() {
    let buf = blocked_cmd(&["FLUSHALL"]).unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$8\r\nFLUSHALL\r\n");
}
#[test]
fn test_command_pttl_encoding() {
    let buf = blocked_cmd(&["PTTL", "mykey"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nPTTL\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_pexpire_encoding() {
    let buf = blocked_cmd(&["PEXPIRE", "mykey", "10000"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$7\r\nPEXPIRE\r\n$5\r\nmykey\r\n$5\r\n10000\r\n"
    );
}
#[test]
fn test_command_pexpireat_encoding() {
    let buf = blocked_cmd(&["PEXPIREAT", "mykey", "1609459200000"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$9\r\nPEXPIREAT\r\n$5\r\nmykey\r\n$13\r\n1609459200000\r\n"
    );
}
#[test]
fn test_command_persist_encoding() {
    let buf = blocked_cmd(&["PERSIST", "mykey"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$7\r\nPERSIST\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_shutdown_encoding() {
    let buf = blocked_cmd(&["SHUTDOWN"]).unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$8\r\nSHUTDOWN\r\n");
}
#[test]
fn test_command_shutdown_nosave_encoding() {
    let buf = blocked_cmd(&["SHUTDOWN", "NOSAVE"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$8\r\nSHUTDOWN\r\n$6\r\nNOSAVE\r\n");
}
#[test]
fn test_command_info_encoding() {
    let buf = blocked_cmd(&["INFO"]).unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nINFO\r\n");
}
#[test]
fn test_command_info_section_encoding() {
    let buf = blocked_cmd(&["INFO", "server"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINFO\r\n$6\r\nserver\r\n");
}
#[test]
fn test_command_config_get_encoding() {
    let buf = blocked_cmd(&["CONFIG", "GET", "maxmemory"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$9\r\nmaxmemory\r\n"
    );
}
#[test]
fn test_command_type_encoding() {
    let buf = blocked_cmd(&["TYPE", "mykey"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nTYPE\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_move_encoding() {
    let buf = blocked_cmd(&["MOVE", "mykey", "1"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nMOVE\r\n$5\r\nmykey\r\n$1\r\n1\r\n"
    );
}
#[test]
fn test_command_rename_encoding() {
    let buf = blocked_cmd(&["RENAME", "mykey", "newkey"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nRENAME\r\n$5\r\nmykey\r\n$6\r\nnewkey\r\n"
    );
}
#[test]
fn test_command_renamemx_encoding() {
    let buf = blocked_cmd(&["RENAMENX", "mykey", "newkey"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$8\r\nRENAMENX\r\n$5\r\nmykey\r\n$6\r\nnewkey\r\n"
    );
}
#[test]
fn test_command_sort_encoding() {
    let buf = blocked_cmd(&["SORT", "mylist"]).unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n");
}
#[test]
fn test_command_sort_limit_encoding() {
    let buf = blocked_cmd(&["SORT", "mylist", "LIMIT", "0", "10"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$2\r\n10\r\n"
    );
}
#[test]
fn test_command_sort_limit_order_encoding() {
    let buf = blocked_cmd(&["SORT", "mylist", "LIMIT", "0", "10", "DESC"]).unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*6\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$2\r\n10\r\n$4\r\nDESC\r\n"
    );
}
