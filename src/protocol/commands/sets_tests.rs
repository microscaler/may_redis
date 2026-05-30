#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::SetsCommands;

#[test]
fn test_command_sadd_encoding() {
    let buf = ().sadd("myset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nSADD\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_sismember_encoding() {
    let buf = ().sismember("myset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$9\r\nSISMEMBER\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_srem_encoding() {
    let buf = ().srem("myset", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nSREM\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_smembers_encoding() {
    let buf = ().smembers("myset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$8\r\nSMEMBERS\r\n$5\r\nmyset\r\n");
}
#[test]
fn test_command_spop_encoding() {
    let buf = ().spop("myset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nSPOP\r\n$5\r\nmyset\r\n");
}
#[test]
fn test_command_spop_count_encoding() {
    let buf = ().spop_count("myset", 3).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$4\r\nSPOP\r\n$5\r\nmyset\r\n$1\r\n3\r\n"
    );
}
#[test]
fn test_command_srandmember_encoding() {
    let buf = ().srandmember("myset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$11\r\nSRANDMEMBER\r\n$5\r\nmyset\r\n");
}
#[test]
fn test_command_srandmember_count_encoding() {
    let buf = ().srandmember_count("myset", 2).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$11\r\nSRANDMEMBER\r\n$5\r\nmyset\r\n$1\r\n2\r\n"
    );
}
#[test]
fn test_command_scard_encoding() {
    let buf = ().scard("myset").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$5\r\nSCARD\r\n$5\r\nmyset\r\n");
}
#[test]
fn test_command_sinter_encoding() {
    let buf = <() as SetsCommands>::sinter(&(), &["set1", "set2"])
        .build()
        .unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nSINTER\r\n$4\r\nset1\r\n$4\r\nset2\r\n"
    );
}
#[test]
fn test_command_sunion_encoding() {
    let buf = <() as SetsCommands>::sunion(&(), &["set1", "set2"])
        .build()
        .unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nSUNION\r\n$4\r\nset1\r\n$4\r\nset2\r\n"
    );
}
#[test]
fn test_command_smove_encoding() {
    let buf = ().smove("src", "dst", "member1").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nSMOVE\r\n$3\r\nsrc\r\n$3\r\ndst\r\n$7\r\nmember1\r\n"
    );
}
#[test]
fn test_command_sscan_encoding() {
    let buf = ().sscan("myset", 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$5\r\nSSCAN\r\n$5\r\nmyset\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_sscan_match_encoding() {
    let buf = ().sscan_match("myset", 0, "m*").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*5\r\n$5\r\nSSCAN\r\n$5\r\nmyset\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$2\r\nm*\r\n"
    );
}
