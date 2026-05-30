// Commands — strings trait
//
// Provides all Strings commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Strings command methods.
pub trait StringsCommands: Sized {
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

    /// DECR key — Decrement the integer value of key by one
    #[must_use = "call .build() to encode the command"]
    fn decr<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("DECR").arg(key)
    }

    /// DECRBY key decrement — Decrement the integer value of key by decrement
    #[must_use = "call .build() to encode the command"]
    fn decrby<K: ToRedisArgs>(&self, key: K, decrement: i64) -> CommandBuilder {
        CommandBuilder::new("DECRBY").arg(key).arg(decrement)
    }

    /// SETNX key value — Set key to value only if key does not exist
    #[must_use = "call .build() to encode the command"]
    fn setnx<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder {
        CommandBuilder::new("SETNX").arg(key).arg(value)
    }

    /// MGET keys — Get the values of all the given keys
    #[must_use = "call .build() to encode the command"]
    fn mget<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("MGET");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }

    /// MSET key value [key value ...] — Set multiple keys to multiple values
    #[must_use = "call .build() to encode the command"]
    fn mset<K: ToRedisArgs, V: ToRedisArgs>(&self, pairs: &[(K, V)]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("MSET");
        for (key, value) in pairs {
            builder = builder.arg(key).arg(value);
        }
        builder
    }

    /// MSETNX key value [key value ...] — Set multiple keys to multiple values, only if none of the keys exist
    #[must_use = "call .build() to encode the command"]
    fn msetnx<K: ToRedisArgs, V: ToRedisArgs>(&self, pairs: &[(K, V)]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("MSETNX");
        for (key, value) in pairs {
            builder = builder.arg(key).arg(value);
        }
        builder
    }

    /// STRLEN key — Get the length of the value stored in key
    #[must_use = "call .build() to encode the command"]
    fn strlen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("STRLEN").arg(key)
    }

    /// GETRANGE key start end — Get a substring of the string stored at key
    #[must_use = "call .build() to encode the command"]
    fn getrange<K: ToRedisArgs>(&self, key: K, start: i64, end: i64) -> CommandBuilder {
        CommandBuilder::new("GETRANGE").arg(key).arg(start).arg(end)
    }

    /// SETBIT key offset value — Sets or clears the bit at offset in the string value stored at key
    #[must_use = "call .build() to encode the command"]
    fn setbit<K: ToRedisArgs>(&self, key: K, offset: i64, value: i64) -> CommandBuilder {
        CommandBuilder::new("SETBIT")
            .arg(key)
            .arg(offset)
            .arg(value)
    }

    /// GETBIT key offset — Returns the bit value at offset in the string value stored at key
    #[must_use = "call .build() to encode the command"]
    fn getbit<K: ToRedisArgs>(&self, key: K, offset: i64) -> CommandBuilder {
        CommandBuilder::new("GETBIT").arg(key).arg(offset)
    }

    /// BITCOUNT key — Count set bits in a string
    #[must_use = "call .build() to encode the command"]
    fn bitcount<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("BITCOUNT").arg(key)
    }

    /// BITCOUNT key start end — Count set bits in a string with byte range
    #[must_use = "call .build() to encode the command"]
    fn bitcount_range<K: ToRedisArgs>(&self, key: K, start: i64, end: i64) -> CommandBuilder {
        CommandBuilder::new("BITCOUNT").arg(key).arg(start).arg(end)
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

    /// SETRANGE key offset value — Overwrite part of a string at key starting at offset
    #[must_use = "call .build() to encode the command"]
    fn setrange<K: ToRedisArgs, V: ToRedisArgs>(
        &self,
        key: K,
        offset: i64,
        value: V,
    ) -> CommandBuilder {
        CommandBuilder::new("SETRANGE")
            .arg(key)
            .arg(offset)
            .arg(value)
    }
}
