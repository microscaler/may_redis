#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::protocol::commands::{
    AdminCommands,
    Commands,
    HashesCommands,
    ListsCommands,
    PubsubCommands,
    SetsCommands,
    SortedSetsCommands,
    StringsCommands,
    TransactionsCommands,
};
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
#[test]
fn test_command_type_encoding() {
let buf = ().type_("mykey").build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nTYPE\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_move_encoding() {
let buf = ().move_key("mykey", 1).build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$4\r\nMOVE\r\n$5\r\nmykey\r\n$1\r\n1\r\n"
);
}
#[test]
fn test_command_rename_encoding() {
let buf = ().rename("mykey", "newkey").build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$6\r\nRENAME\r\n$5\r\nmykey\r\n$6\r\nnewkey\r\n"
);
}
#[test]
fn test_command_renamemx_encoding() {
let buf = ().renamemx("mykey", "newkey").build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$8\r\nRENAMENX\r\n$5\r\nmykey\r\n$6\r\nnewkey\r\n"
);
}
#[test]
fn test_command_sort_encoding() {
let buf = ().sort("mylist").build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n");
}
#[test]
fn test_command_sort_limit_encoding() {
let buf = ().sort_limit("mylist", 0, 10).build().unwrap();
assert_eq!(
buf.as_ref(),
b"*5\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$2\r\n10\r\n"
);
}
#[test]
fn test_command_sort_limit_order_encoding() {
let buf = ().sort_limit_order("mylist", 0, 10, "DESC").build().unwrap();
assert_eq!(
buf.as_ref(),
b"*6\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$2\r\n10\r\n$4\r\nDESC\r\n"
);
}
#[test]
fn test_command_scan_encoding() {
let buf = ().scan(0).build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nSCAN\r\n$1\r\n0\r\n");
}
#[test]
fn test_command_scan_match_encoding() {
let buf = ().scan_match(0, "foo*").build().unwrap();
assert_eq!(
buf.as_ref(),
b"*4\r\n$4\r\nSCAN\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$4\r\nfoo*\r\n"
);
}
#[test]
fn test_command_touch_encoding() {
let buf = ().touch(&["k1", "k2", "k3"]).build().unwrap();
assert_eq!(
buf.as_ref(),
b"*4\r\n$5\r\nTOUCH\r\n$2\r\nk1\r\n$2\r\nk2\r\n$2\r\nk3\r\n"
);
}
#[test]
fn test_command_save_encoding() {
let buf = ().save().build().unwrap();
assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nSAVE\r\n");
}
#[test]
fn test_command_bgsave_encoding() {
let buf = ().bgsave().build().unwrap();
assert_eq!(buf.as_ref(), b"*1\r\n$6\r\nBGSAVE\r\n");
}
#[test]
fn test_command_flushall_encoding() {
let buf = ().flushall().build().unwrap();
assert_eq!(buf.as_ref(), b"*1\r\n$8\r\nFLUSHALL\r\n");
}
#[test]
fn test_command_pttl_encoding() {
let buf = ().pttl("mykey").build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nPTTL\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_pexpire_encoding() {
let buf = ().pexpire("mykey", 10000).build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$7\r\nPEXPIRE\r\n$5\r\nmykey\r\n$5\r\n10000\r\n"
);
}
#[test]
fn test_command_pexpireat_encoding() {
let buf = ().pexpireat("mykey", 1_609_459_200_000).build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$9\r\nPEXPIREAT\r\n$5\r\nmykey\r\n$13\r\n1609459200000\r\n"
);
}
#[test]
fn test_command_persist_encoding() {
let buf = ().persist("mykey").build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$7\r\nPERSIST\r\n$5\r\nmykey\r\n");
}
#[test]
fn test_command_shutdown_encoding() {
let buf = ().shutdown().build().unwrap();
assert_eq!(buf.as_ref(), b"*1\r\n$8\r\nSHUTDOWN\r\n");
}
#[test]
fn test_command_shutdown_nosave_encoding() {
let buf = ().shutdown_nosave().build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$8\r\nSHUTDOWN\r\n$6\r\nNOSAVE\r\n");
}
#[test]
fn test_command_info_encoding() {
let buf = ().info().build().unwrap();
assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nINFO\r\n");
}
#[test]
fn test_command_info_server_encoding() {
let buf = ().info_section("server").build().unwrap();
assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINFO\r\n$6\r\nserver\r\n");
}
#[test]
fn test_command_config_get_encoding() {
let buf = ().config_get("maxmemory").build().unwrap();
assert_eq!(
buf.as_ref(),
b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$9\r\nmaxmemory\r\n"
);
}