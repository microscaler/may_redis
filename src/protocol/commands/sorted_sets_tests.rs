#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::SortedSetsCommands;

#[test]
fn test_command_zadd_encoding() {
    let buf = ().zadd("myzset", 1.0, "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nZADD\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_zadd_multi_encoding() {
    let buf = ().zadd_multi("myzset", &[(1.0, "m1"), (2.0, "m2")]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*6\r\n$4\r\nZADD\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$2\r\nm1\r\n$3\r\n2.0\r\n$2\r\nm2\r\n"
    );
}
#[test]
fn test_command_zrem_encoding() {
    let buf = ().zrem("myzset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nZREM\r\n$6\r\nmyzset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_zrem_members_encoding() {
    let buf = ().zrem_members("myzset", &["m1", "m2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nZREM\r\n$6\r\nmyzset\r\n$2\r\nm1\r\n$2\r\nm2\r\n"
    );
}
#[test]
fn test_command_zrange_encoding() {
    let buf = ().zrange("myzset", 0, -1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$6\r\nZRANGE\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
}
#[test]
fn test_command_zrange_withscores_encoding() {
    let buf = ().zrange_withscores("myzset", 0, -1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$6\r\nZRANGE\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$2\r\n-1\r\n$10\r\nWITHSCORES\r\n"
    );
}
#[test]
fn test_command_zrank_encoding() {
    let buf = ().zrank("myzset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nZRANK\r\n$6\r\nmyzset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_zscore_encoding() {
    let buf = ().zscore("myzset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nZSCORE\r\n$6\r\nmyzset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_zcard_encoding() {
    let buf = ().zcard("myzset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$5\r\nZCARD\r\n$6\r\nmyzset\r\n");
}
#[test]
fn test_command_zcount_encoding() {
    let buf = ().zcount("myzset", 1.0, 10.0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$6\r\nZCOUNT\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n"
    );
}
#[test]
fn test_command_zincrby_encoding() {
    let buf = ().zincrby("myzset", 5.0, "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$7\r\nZINCRBY\r\n$6\r\nmyzset\r\n$3\r\n5.0\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_zpopmax_encoding() {
    let buf = ().zpopmax("myzset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$7\r\nZPOPMAX\r\n$6\r\nmyzset\r\n");
}
#[test]
fn test_command_zpopmax_count_encoding() {
    let buf = ().zpopmax_count("myzset", 3).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$7\r\nZPOPMAX\r\n$6\r\nmyzset\r\n$1\r\n3\r\n"
    );
}
#[test]
fn test_command_zpopmin_encoding() {
    let buf = ().zpopmin("myzset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$7\r\nZPOPMIN\r\n$6\r\nmyzset\r\n");
}
#[test]
fn test_command_zpopmin_count_encoding() {
    let buf = ().zpopmin_count("myzset", 3).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$7\r\nZPOPMIN\r\n$6\r\nmyzset\r\n$1\r\n3\r\n"
    );
}
#[test]
fn test_command_zscan_encoding() {
    let buf = ().zscan("myzset", 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nZSCAN\r\n$6\r\nmyzset\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_zscan_match_encoding() {
    let buf = ().zscan_match("myzset", 0, "m*").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$5\r\nZSCAN\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$2\r\nm*\r\n"
    );
}
#[test]
fn test_command_zrangebyscore_encoding() {
    let buf = ().zrangebyscore("myzset", 1.0, 10.0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n"
    );
}
#[test]
fn test_command_zrangebyscore_withscores_encoding() {
    let buf = ().zrangebyscore_withscores("myzset", 1.0, 10.0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n$10\r\nWITHSCORES\r\n"
    );
}
#[test]
fn test_command_zrangebyscore_limit_encoding() {
    let buf = ().zrangebyscore_limit("myzset", 1.0, 10.0, 0, 5).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*7\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$1\r\n5\r\n"
    );
}
