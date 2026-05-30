// Commands — hashes trait
//
// Provides all Hashes commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Hashes command methods.
pub trait HashesCommands: Sized {

    /// HGET key field
    #[must_use = "call .build() to encode the command"]
    fn hget<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder {
        CommandBuilder::new("HGET").arg(key).arg(field)
    }


    /// HDEL key field [field ...] — Delete one or more hash fields
    #[must_use = "call .build() to encode the command"]
    fn hdel<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder {
        CommandBuilder::new("HDEL").arg(key).arg(field)
    }


    /// HDEL fields — Delete multiple hash fields (variadic)
    #[must_use = "call .build() to encode the command"]
    fn hdel_fields<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, fields: &[F]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("HDEL");
        builder = builder.arg(key);
        for f in fields {
            builder = builder.arg(f);
        }
        builder
    }


    /// HKEYS key — Get all field names in a hash
    #[must_use = "call .build() to encode the command"]
    fn hkeys<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("HKEYS").arg(key)
    }


    /// HGETALL key — Get all fields and values in a hash
    #[must_use = "call .build() to encode the command"]
    fn hgetall<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("HGETALL").arg(key)
    }


    /// HLEN key — Get the number of fields in a hash
    #[must_use = "call .build() to encode the command"]
    fn hlen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("HLEN").arg(key)
    }


    /// HEXISTS key field — Check if a hash field exists
    #[must_use = "call .build() to encode the command"]
    fn hexists<K: ToRedisArgs, F: ToRedisArgs>(&self, key: K, field: F) -> CommandBuilder {
        CommandBuilder::new("HEXISTS").arg(key).arg(field)
    }


    /// HSCAN key cursor — Incrementally iterate hash fields and values
    #[must_use = "call .build() to encode the command"]
    fn hscan<K: ToRedisArgs>(&self, key: K, cursor: i64) -> CommandBuilder {
        CommandBuilder::new("HSCAN").arg(key).arg(cursor)
    }


    /// HSCAN MATCH key cursor pattern — Incrementally iterate hash fields matching a pattern
    #[must_use = "call .build() to encode the command"]
    fn hscan_match<K: ToRedisArgs>(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder {
        CommandBuilder::new("HSCAN")
            .arg(key)
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
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


    /// HMSET key field value [field value ...] — Set multiple hash fields to multiple values
    #[must_use = "call .build() to encode the command"]
    fn hmset<K: ToRedisArgs, V: ToRedisArgs>(
        &self,
        key: K,
        pairs: &[(impl ToRedisArgs, V)],
    ) -> CommandBuilder {
        let mut builder = CommandBuilder::new("HMSET");
        builder = builder.arg(key);
        for (field, value) in pairs {
            builder = builder.arg(field).arg(value);
        }
        builder
    }


    /// HINCRBY key field increment — Increment the integer value of a hash field by increment
    #[must_use = "call .build() to encode the command"]
    fn hincrby<K: ToRedisArgs, F: ToRedisArgs>(
        &self,
        key: K,
        field: F,
        increment: i64,
    ) -> CommandBuilder {
        CommandBuilder::new("HINCRBY")
            .arg(key)
            .arg(field)
            .arg(increment)
    }

}
