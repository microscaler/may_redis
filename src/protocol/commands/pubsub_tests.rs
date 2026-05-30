#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::PubsubCommands;

#[test]
fn test_command_subscribe_encoding() {
    let buf = ().subscribe(&["ch1", "ch2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$9\r\nSUBSCRIBE\r\n$3\r\nch1\r\n$3\r\nch2\r\n"
    );
}
#[test]
fn test_command_unsubscribe_encoding() {
    let buf = ().unsubscribe().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$11\r\nUNSUBSCRIBE\r\n");
}
#[test]
fn test_command_unsubscribe_channels_encoding() {
    let buf = ().unsubscribe_channels(&["ch1", "ch2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$11\r\nUNSUBSCRIBE\r\n$3\r\nch1\r\n$3\r\nch2\r\n"
    );
}
#[test]
fn test_command_psubscribe_encoding() {
    let buf = ().psubscribe(&["pattern*", "test?*"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$10\r\nPSUBSCRIBE\r\n$8\r\npattern*\r\n$6\r\ntest?*\r\n"
    );
}
#[test]
fn test_command_punsubscribe_encoding() {
    let buf = ().punsubscribe().build().unwrap();
    assert_eq!(buf.as_ref(), b"*1\r\n$12\r\nPUNSUBSCRIBE\r\n");
}
#[test]
fn test_command_punsubscribe_patterns_encoding() {
    let buf = ().punsubscribe_patterns(&["pattern*", "test?*"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$12\r\nPUNSUBSCRIBE\r\n$8\r\npattern*\r\n$6\r\ntest?*\r\n"
    );
}
