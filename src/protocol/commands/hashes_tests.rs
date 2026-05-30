#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::HashesCommands;

#[test]
fn test_command_hset_encoding() {
    let buf = ().hset("myhash", "field1", "value1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nHSET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n$6\r\nvalue1\r\n"
    );
}
#[test]
fn test_command_hget_encoding() {
    let buf = ().hget("myhash", "field1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nHGET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
    );
}
#[test]
fn test_command_hdel_encoding() {
    let buf = ().hdel("myhash", "field1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nHDEL\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
    );
}
#[test]
fn test_command_hdel_fields_encoding() {
    let buf = ().hdel_fields("myhash", &["f1", "f2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nHDEL\r\n$6\r\nmyhash\r\n$2\r\nf1\r\n$2\r\nf2\r\n"
    );
}
#[test]
fn test_command_hkeys_encoding() {
    let buf = ().hkeys("myhash").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$5\r\nHKEYS\r\n$6\r\nmyhash\r\n");
}
#[test]
fn test_command_hgetall_encoding() {
    let buf = ().hgetall("myhash").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$7\r\nHGETALL\r\n$6\r\nmyhash\r\n");
}
#[test]
fn test_command_hmset_encoding() {
    let buf = ().hmset("myhash", &[("f1", "v1"), ("f2", "v2")]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*6\r\n$5\r\nHMSET\r\n$6\r\nmyhash\r\n$2\r\nf1\r\n$2\r\nv1\r\n$2\r\nf2\r\n$2\r\nv2\r\n"
    );
}
#[test]
fn test_command_hincrby_encoding() {
    let buf = ().hincrby("myhash", "counter", 5).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$7\r\nHINCRBY\r\n$6\r\nmyhash\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
    );
}
#[test]
fn test_command_hlen_encoding() {
    let buf = ().hlen("myhash").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nHLEN\r\n$6\r\nmyhash\r\n");
}
#[test]
fn test_command_hexists_encoding() {
    let buf = ().hexists("myhash", "field1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$7\r\nHEXISTS\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
    );
}
#[test]
fn test_command_hscan_encoding() {
    let buf = ().hscan("myhash", 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nHSCAN\r\n$6\r\nmyhash\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_hscan_match_encoding() {
    let buf = ().hscan_match("myhash", 0, "f*").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$5\r\nHSCAN\r\n$6\r\nmyhash\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$2\r\nf*\r\n"
    );
}
