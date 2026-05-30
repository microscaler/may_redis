// Commands — sets trait
//
// Provides all Sets commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Sets command methods.
pub trait SetsCommands: Sized {

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


    /// SMEMBERS key — Get all members in a set
    #[must_use = "call .build() to encode the command"]
    fn smembers<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("SMEMBERS").arg(key)
    }


    /// SPOP key — Remove and return a random member from a set
    #[must_use = "call .build() to encode the command"]
    fn spop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("SPOP").arg(key)
    }


    /// SPOP key count — Remove and return up to count random members from a set
    #[must_use = "call .build() to encode the command"]
    fn spop_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder {
        CommandBuilder::new("SPOP").arg(key).arg(count)
    }


    /// SRANDMEMBER key — Return a random member from a set
    #[must_use = "call .build() to encode the command"]
    fn srandmember<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("SRANDMEMBER").arg(key)
    }


    /// SRANDMEMBER key count — Return up to count random members from a set
    #[must_use = "call .build() to encode the command"]
    fn srandmember_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder {
        CommandBuilder::new("SRANDMEMBER").arg(key).arg(count)
    }


    /// SCARD key — Get the number of members in a set
    #[must_use = "call .build() to encode the command"]
    fn scard<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("SCARD").arg(key)
    }


    /// SINTER keys — Get the intersection of multiple sets
    #[must_use = "call .build() to encode the command"]
    fn sinter<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("SINTER");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }


    /// SUNION keys — Get the union of multiple sets
    #[must_use = "call .build() to encode the command"]
    fn sunion<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("SUNION");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }


    /// SSCAN key cursor — Incrementally iterate set members
    #[must_use = "call .build() to encode the command"]
    fn sscan<K: ToRedisArgs>(&self, key: K, cursor: i64) -> CommandBuilder {
        CommandBuilder::new("SSCAN").arg(key).arg(cursor)
    }


    /// SSCAN MATCH key cursor pattern — Incrementally iterate set members matching a pattern
    #[must_use = "call .build() to encode the command"]
    fn sscan_match<K: ToRedisArgs>(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder {
        CommandBuilder::new("SSCAN")
            .arg(key)
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
    }



    /// SMOVE source destination member — Move a member from one set to another
    #[must_use = "call .build() to encode the command"]
    fn smove<K: ToRedisArgs, M: ToRedisArgs>(
        &self,
        source: K,
        destination: K,
        member: M,
    ) -> CommandBuilder {
        CommandBuilder::new("SMOVE")
            .arg(source)
            .arg(destination)
            .arg(member)
    }

}
