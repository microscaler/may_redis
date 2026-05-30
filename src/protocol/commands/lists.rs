// Commands — lists trait
//
// Provides all Lists commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Lists command methods.
pub trait ListsCommands: Sized {

    /// LPUSH key values — Prepend one or multiple values to a list
    #[must_use = "call .build() to encode the command"]
    fn lpush<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, values: &[V]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("LPUSH");
        builder = builder.arg(key);
        for v in values {
            builder = builder.arg(v);
        }
        builder
    }


    /// RPUSH key values — Append one or multiple values to a list
    #[must_use = "call .build() to encode the command"]
    fn rpush<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, values: &[V]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("RPUSH");
        builder = builder.arg(key);
        for v in values {
            builder = builder.arg(v);
        }
        builder
    }


    /// LPOP key — Remove and return the first element of a list
    #[must_use = "call .build() to encode the command"]
    fn lpop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("LPOP").arg(key)
    }


    /// RPOP key — Remove and return the last element of a list
    #[must_use = "call .build() to encode the command"]
    fn rpop<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("RPOP").arg(key)
    }


    /// LLEN key — Get the length of a list
    #[must_use = "call .build() to encode the command"]
    fn llen<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("LLEN").arg(key)
    }


    /// LRANGE key start stop — Get a range of elements from a list
    #[must_use = "call .build() to encode the command"]
    fn lrange<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder {
        CommandBuilder::new("LRANGE").arg(key).arg(start).arg(stop)
    }


    /// LINDEX key index — Get an element from a list by its index
    #[must_use = "call .build() to encode the command"]
    fn lindex<K: ToRedisArgs>(&self, key: K, index: i64) -> CommandBuilder {
        CommandBuilder::new("LINDEX").arg(key).arg(index)
    }


    /// LSET key index value — Set the value of an element in a list by its index
    #[must_use = "call .build() to encode the command"]
    fn lset<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, index: i64, value: V) -> CommandBuilder {
        CommandBuilder::new("LSET").arg(key).arg(index).arg(value)
    }


    /// LREM key count value — Remove elements matching a value from a list
    #[must_use = "call .build() to encode the command"]
    fn lrem<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, count: i64, value: V) -> CommandBuilder {
        CommandBuilder::new("LREM").arg(key).arg(count).arg(value)
    }


    /// LTRIM key start stop — Trim a list to the specified range
    #[must_use = "call .build() to encode the command"]
    fn ltrim<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder {
        CommandBuilder::new("LTRIM").arg(key).arg(start).arg(stop)
    }


    /// BLPOP keys timeout — Remove and get the first element from a list, or block until one is available.
    ///
    /// # Timeout semantics
    ///
    /// * `timeout == 0`: Block **indefinitely** until a list element is available.
    ///   The default `execute()` method has a 30-second timeout, so calling this
    ///   with `timeout == 0` will **always** return `RedisError::Connection` after
    ///   30 seconds. Use `client.execute_timeout(cmd, 60)` (or similar) to wait
    ///   longer.
    /// * `timeout > 0`: Block for at most `timeout` seconds. If no element is
    ///   available, returns `None` (converted from Redis's nil bulk string).
    ///
    /// # Warning: default timeout may interfere
    ///
    /// `RedisClient::execute()` uses a 30-second timeout internally. For
    /// blocking commands with `timeout > 0`, the Redis-side timeout must be
    /// **strictly less** than the client-side 30-second timeout, or the
    /// client will cancel the request before Redis can respond.
    #[must_use = "call .build() to encode the command"]
    fn blpop<K: ToRedisArgs>(&self, keys: &[K], timeout: i64) -> CommandBuilder {
        let mut builder = CommandBuilder::new("BLPOP");
        for key in keys {
            builder = builder.arg(key);
        }
        builder.arg(timeout)
    }


    /// BRPOP keys timeout — Remove and get the last element from a list, or block until one is available
    #[must_use = "call .build() to encode the command"]
    fn brpop<K: ToRedisArgs>(&self, keys: &[K], timeout: i64) -> CommandBuilder {
        let mut builder = CommandBuilder::new("BRPOP");
        for key in keys {
            builder = builder.arg(key);
        }
        builder.arg(timeout)
    }

}
