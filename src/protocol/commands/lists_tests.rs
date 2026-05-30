#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::protocol::commands::ListsCommands;

#[test]
fn test_command_lpush_encoding() {
    let buf = ().lpush("mylist", &["v1", "v2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nLPUSH\r\n$6\r\nmylist\r\n$2\r\nv1\r\n$2\r\nv2\r\n"
    );
}
#[test]
fn test_command_rpush_encoding() {
    let buf = ().rpush("mylist", &["v1", "v2"]).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nRPUSH\r\n$6\r\nmylist\r\n$2\r\nv1\r\n$2\r\nv2\r\n"
    );
}
#[test]
fn test_command_lpop_encoding() {
    let buf = ().lpop("mylist").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nLPOP\r\n$6\r\nmylist\r\n");
}
#[test]
fn test_command_rpop_encoding() {
    let buf = ().rpop("mylist").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nRPOP\r\n$6\r\nmylist\r\n");
}
#[test]
fn test_command_llen_encoding() {
    let buf = ().llen("mylist").build().unwrap();
    assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nLLEN\r\n$6\r\nmylist\r\n");
}
#[test]
fn test_command_lrange_encoding() {
    let buf = ().lrange("mylist", 0, -1).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$6\r\nLRANGE\r\n$6\r\nmylist\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
}
#[test]
fn test_command_lindex_encoding() {
    let buf = ().lindex("mylist", 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*3\r\n$6\r\nLINDEX\r\n$6\r\nmylist\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_lset_encoding() {
    let buf = ().lset("mylist", 0, "v").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nLSET\r\n$6\r\nmylist\r\n$1\r\n0\r\n$1\r\nv\r\n"
    );
}
#[test]
fn test_command_lrem_encoding() {
    let buf = ().lrem("mylist", 0, "v").build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$4\r\nLREM\r\n$6\r\nmylist\r\n$1\r\n0\r\n$1\r\nv\r\n"
    );
}
#[test]
fn test_command_ltrim_encoding() {
    let buf = ().ltrim("mylist", 0, 10).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nLTRIM\r\n$6\r\nmylist\r\n$1\r\n0\r\n$2\r\n10\r\n"
    );
}
#[test]
fn test_command_blpop_encoding() {
    let buf = ().blpop(&["list1", "list2"], 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nBLPOP\r\n$5\r\nlist1\r\n$5\r\nlist2\r\n$1\r\n0\r\n"
    );
}
#[test]
fn test_command_brpop_encoding() {
    let buf = ().brpop(&["list1", "list2"], 0).build().unwrap();
    assert_eq!(
        buf.as_ref(),
        b"*4\r\n$5\r\nBRPOP\r\n$5\r\nlist1\r\n$5\r\nlist2\r\n$1\r\n0\r\n"
    );
}
