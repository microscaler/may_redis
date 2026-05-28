// Commands — Trait mirroring the redis crate API surface.

use crate::core::ToRedisArgs;

use super::builder::CommandBuilder;

/// Trait that provides all Redis command methods.
///
/// Each method constructs a `CommandBuilder` for a specific Redis command,
/// which can then be encoded into RESP2 wire format via [`build()`](CommandBuilder::build).
pub trait Commands: Sized {
    /// GET key
    #[must_use = "call .build() to encode the command"]
    fn get<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("GET").arg(key)
    }

    /// SET key value
    #[must_use = "call .build() to encode the command"]
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder {
        CommandBuilder::new("SET").arg(key).arg(value)
    }

    /// SET key value EX seconds
    #[must_use = "call .build() to encode the command"]
    fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(
        &self,
        key: K,
        value: V,
        seconds: u32,
    ) -> CommandBuilder {
        CommandBuilder::new("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(seconds)
    }

    /// EXISTS key
    #[must_use = "call .build() to encode the command"]
    fn exists<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("EXISTS").arg(key)
    }

    /// DEL key
    #[must_use = "call .build() to encode the command"]
    fn del<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("DEL").arg(key)
    }

    /// INCR key
    #[must_use = "call .build() to encode the command"]
    fn incr<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("INCR").arg(key)
    }

    /// TTL key
    #[must_use = "call .build() to encode the command"]
    fn ttl<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("TTL").arg(key)
    }

    /// EXPIRE key seconds
    #[must_use = "call .build() to encode the command"]
    fn expire<K: ToRedisArgs>(&self, key: K, seconds: u32) -> CommandBuilder {
        CommandBuilder::new("EXPIRE").arg(key).arg(seconds)
    }

    /// PUBLISH channel message
    #[must_use = "call .build() to encode the command"]
    fn publish<K: ToRedisArgs, M: ToRedisArgs>(&self, channel: K, message: M) -> CommandBuilder {
        CommandBuilder::new("PUBLISH").arg(channel).arg(message)
    }

    /// KEYS pattern
    #[must_use = "call .build() to encode the command"]
    fn keys<K: ToRedisArgs>(&self, pattern: K) -> CommandBuilder {
        CommandBuilder::new("KEYS").arg(pattern)
    }

    /// DBSIZE
    #[must_use = "call .build() to encode the command"]
    fn dbsize(&self) -> CommandBuilder {
        CommandBuilder::new("DBSIZE")
    }

    /// FLUSHDB
    #[must_use = "call .build() to encode the command"]
    fn flushdb(&self) -> CommandBuilder {
        CommandBuilder::new("FLUSHDB")
    }

    /// PING
    #[must_use = "call .build() to encode the command"]
    fn ping(&self) -> CommandBuilder {
        CommandBuilder::new("PING")
    }

    /// AUTH password
    #[must_use = "call .build() to encode the command"]
    fn auth(&self, password: &str) -> CommandBuilder {
        CommandBuilder::new("AUTH").arg(password)
    }

    /// HSET key field value [field value ...]
    #[must_use = "call .build() to encode the command"]
    fn hset<K: ToRedisArgs, F: ToRedisArgs, V: ToRedisArgs>(
        &self,
        key: K,
        field: F,
        value: V,
    ) -> CommandBuilder {
        CommandBuilder::new("HSET").arg(key).arg(field).arg(value)
    }

    /// HGET key field
    #[must_use = "call .build() to encode the command"]
    fn hget<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder {
        CommandBuilder::new("HGET").arg(key).arg(field)
    }

    /// SADD key member [member ...]
    #[must_use = "call .build() to encode the command"]
    fn sadd<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("SADD").arg(key).arg(member)
    }

    /// SISMEMBER key member
    #[must_use = "call .build() to encode the command"]
    fn sismember<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("SISMEMBER").arg(key).arg(member)
    }

    /// SREM key member [member ...]
    #[must_use = "call .build() to encode the command"]
    fn srem<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("SREM").arg(key).arg(member)
    }

    /// SETEX key seconds value
    #[must_use = "call .build() to encode the command"]
    fn setex<K: ToRedisArgs, V: ToRedisArgs>(
        &self,
        key: K,
        seconds: u32,
        value: V,
    ) -> CommandBuilder {
        CommandBuilder::new("SETEX")
            .arg(key)
            .arg(seconds)
            .arg(value)
    }

    /// INCRBY key increment
    #[must_use = "call .build() to encode the command"]
    fn incrby<K: ToRedisArgs>(&self, key: K, increment: i64) -> CommandBuilder {
        CommandBuilder::new("INCRBY").arg(key).arg(increment)
    }

    /// APPEND key value
    #[must_use = "call .build() to encode the command"]
    fn append<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder {
        CommandBuilder::new("APPEND").arg(key).arg(value)
    }
}

// Blanket impl so () implements Commands
impl Commands for () {}

#[cfg(test)]
mod tests {
    use super::Commands;

    #[test]
    fn test_command_get_encoding() {
        let buf = ().get("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_command_set_encoding() {
        let buf = ().set("key", "val").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n"
        );
    }

    #[test]
    fn test_command_set_ex_encoding() {
        let buf = ().set_ex("key", "val", 60).build();
        assert_eq!(
            buf.as_ref(),
            b"*5\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n$2\r\nEX\r\n$2\r\n60\r\n"
        );
    }

    #[test]
    fn test_command_exists_encoding() {
        let buf = ().exists("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$6\r\nEXISTS\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_command_del_encoding() {
        let buf = ().del("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nDEL\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_command_incr_encoding() {
        let buf = ().incr("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINCR\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_command_ttl_encoding() {
        let buf = ().ttl("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nTTL\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_command_expire_encoding() {
        let buf = ().expire("key", 60).build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$6\r\nEXPIRE\r\n$3\r\nkey\r\n$2\r\n60\r\n"
        );
    }

    #[test]
    fn test_command_publish_encoding() {
        let buf = ().publish("channel", "message").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$7\r\nPUBLISH\r\n$7\r\nchannel\r\n$7\r\nmessage\r\n"
        );
    }

    #[test]
    fn test_command_keys_encoding() {
        let buf = ().keys("*").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n");
    }

    #[test]
    fn test_command_dbsize_encoding() {
        let buf = ().dbsize().build();
        assert_eq!(buf.as_ref(), b"*1\r\n$6\r\nDBSIZE\r\n");
    }

    #[test]
    fn test_command_flushdb_encoding() {
        let buf = ().flushdb().build();
        assert_eq!(buf.as_ref(), b"*1\r\n$7\r\nFLUSHDB\r\n");
    }

    #[test]
    fn test_command_ping_encoding() {
        let buf = ().ping().build();
        assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nPING\r\n");
    }

    #[test]
    fn test_command_auth_encoding() {
        let buf = ().auth("secret").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nAUTH\r\n$6\r\nsecret\r\n");
    }

    #[test]
    fn test_command_hset_encoding() {
        let buf = ().hset("myhash", "field1", "value1").build();
        assert_eq!(
            buf.as_ref(),
            b"*4\r\n$4\r\nHSET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n$6\r\nvalue1\r\n"
        );
    }

    #[test]
    fn test_command_hget_encoding() {
        let buf = ().hget("myhash", "field1").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$4\r\nHGET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
        );
    }

    #[test]
    fn test_command_sadd_encoding() {
        let buf = ().sadd("myset", "member1").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$4\r\nSADD\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_sismember_encoding() {
        let buf = ().sismember("myset", "member1").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$9\r\nSISMEMBER\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_srem_encoding() {
        let buf = ().srem("myset", "member1").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$4\r\nSREM\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_setex_encoding() {
        let buf = ().setex("key", 60, "val").build();
        assert_eq!(
            buf.as_ref(),
            b"*4\r\n$5\r\nSETEX\r\n$3\r\nkey\r\n$2\r\n60\r\n$3\r\nval\r\n"
        );
    }

    #[test]
    fn test_command_incrby_encoding() {
        let buf = ().incrby("counter", 5).build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$6\r\nINCRBY\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
        );
    }

    #[test]
    fn test_command_append_encoding() {
        let buf = ().append("key", "hello").build();
        assert_eq!(
            buf.as_ref(),
            b"*3\r\n$6\r\nAPPEND\r\n$3\r\nkey\r\n$5\r\nhello\r\n"
        );
    }
}
