#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::admin::AdminCommands;
use crate::protocol::commands::TransactionsCommands;

#[test]
fn test_command_multi_encoding() {
    let buf = ().multi().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$5\r\nMULTI\r\n");
}
#[test]
fn test_command_exec_encoding() {
    let buf = ().exec().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nEXEC\r\n");
}
#[test]
fn test_command_discard_encoding() {
    let buf = ().discard().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$7\r\nDISCARD\r\n");
}
#[test]
fn test_command_watch_encoding() {
    let buf = ().watch(&["key1", "key2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nWATCH\r\n$4\r\nkey1\r\n$4\r\nkey2\r\n"
    );
}
#[test]
fn test_command_unwatch_encoding() {
    let buf = ().unwatch().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$7\r\nUNWATCH\r\n");
}
#[test]
fn test_command_select_encoding() {
    let buf = ().select(1).build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$6\r\nSELECT\r\n$1\r\n1\r\n");
}
