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

    /// PUBLISH channel message — Publish a message to a channel.
    ///
    /// # Warning: pub/sub requires a dedicated connection
    ///
    /// `SUBSCRIBE`, `PSUBSCRIBE`, `UNSUBSCRIBE`, and `PUNSUBSCRIBE` put the
    /// connection into a *subscription state* where the Redis server sends
    /// unsolicited messages (the published payloads) to the client. This
    /// means:
    /// - The connection loop **must** handle both request-response messages
    ///   and incoming pub/sub messages on the same socket.
    /// - **This client does not yet support pub/sub.** A subscribe call will
    ///   put the connection in a state where normal request-response
    ///   correlation breaks, because messages arrive out of order.
    /// - **Do NOT use** `subscribe`, `psubscribe`, `unsubscribe`, or
    ///   `punsubscribe` with this client. They will likely cause data loss
    ///   or deadlocks.
    ///
    /// `PUBLISH` (fire-and-forget) is safe because it does not change the
    /// connection state — it just sends a command and returns the number
    /// of subscribers.
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

    /// ZADD key score member — Add a member with a score to a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zadd<K: ToRedisArgs, M: ToRedisArgs>(
        &self,
        key: K,
        score: f64,
        member: M,
    ) -> CommandBuilder {
        CommandBuilder::new("ZADD").arg(key).arg(score).arg(member)
    }

    /// ZADD key scores — Add multiple members with scores to a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zadd_multi<K: ToRedisArgs, M: ToRedisArgs>(
        &self,
        key: K,
        scores: &[(f64, M)],
    ) -> CommandBuilder {
        let mut builder = CommandBuilder::new("ZADD");
        builder = builder.arg(key);
        for (score, member) in scores {
            builder = builder.arg(score).arg(member);
        }
        builder
    }

    /// ZREM key member — Remove a member from a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zrem<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("ZREM").arg(key).arg(member)
    }

    /// ZREM key members — Remove multiple members from a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zrem_members<K: ToRedisArgs, M: ToRedisArgs>(
        &self,
        key: K,
        members: &[M],
    ) -> CommandBuilder {
        let mut builder = CommandBuilder::new("ZREM");
        builder = builder.arg(key);
        for member in members {
            builder = builder.arg(member);
        }
        builder
    }

    /// ZRANGE key start stop — Return a range of members in a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zrange<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder {
        CommandBuilder::new("ZRANGE").arg(key).arg(start).arg(stop)
    }

    /// ZRANGE key start stop WITHSCORES — Return a range with scores
    #[must_use = "call .build() to encode the command"]
    fn zrange_withscores<K: ToRedisArgs>(&self, key: K, start: i64, stop: i64) -> CommandBuilder {
        CommandBuilder::new("ZRANGE")
            .arg(key)
            .arg(start)
            .arg(stop)
            .arg("WITHSCORES")
    }

    /// ZRANK key member — Return the rank of a member in a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zrank<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("ZRANK").arg(key).arg(member)
    }

    /// ZSCORE key member — Return the score of a member in a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zscore<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("ZSCORE").arg(key).arg(member)
    }

    /// ZCARD key — Return the number of members in a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zcard<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("ZCARD").arg(key)
    }

    /// ZCOUNT key min max — Count members in a sorted set by score
    #[must_use = "call .build() to encode the command"]
    fn zcount<K: ToRedisArgs>(&self, key: K, min: f64, max: f64) -> CommandBuilder {
        CommandBuilder::new("ZCOUNT").arg(key).arg(min).arg(max)
    }

    /// ZINCRBY key increment member — Increment the score of a member
    #[must_use = "call .build() to encode the command"]
    fn zincrby<K: ToRedisArgs, M: ToRedisArgs>(
        &self,
        key: K,
        increment: f64,
        member: M,
    ) -> CommandBuilder {
        CommandBuilder::new("ZINCRBY")
            .arg(key)
            .arg(increment)
            .arg(member)
    }

    /// ZPOPMAX key — Remove and return the member with the highest score
    #[must_use = "call .build() to encode the command"]
    fn zpopmax<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("ZPOPMAX").arg(key)
    }

    /// ZPOPMAX key count — Remove and return up to count members with highest scores
    #[must_use = "call .build() to encode the command"]
    fn zpopmax_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder {
        CommandBuilder::new("ZPOPMAX").arg(key).arg(count)
    }

    /// ZPOPMIN key — Remove and return the member with the lowest score
    #[must_use = "call .build() to encode the command"]
    fn zpopmin<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("ZPOPMIN").arg(key)
    }

    /// ZPOPMIN key count — Remove and return up to count members with lowest scores
    #[must_use = "call .build() to encode the command"]
    fn zpopmin_count<K: ToRedisArgs>(&self, key: K, count: i64) -> CommandBuilder {
        CommandBuilder::new("ZPOPMIN").arg(key).arg(count)
    }

    /// ZSCAN key cursor — Incrementally iterate sorted set members
    #[must_use = "call .build() to encode the command"]
    fn zscan<K: ToRedisArgs>(&self, key: K, cursor: i64) -> CommandBuilder {
        CommandBuilder::new("ZSCAN").arg(key).arg(cursor)
    }

    /// ZSCAN MATCH key cursor pattern — Incrementally iterate with pattern matching
    #[must_use = "call .build() to encode the command"]
    fn zscan_match<K: ToRedisArgs>(&self, key: K, cursor: i64, pattern: &str) -> CommandBuilder {
        CommandBuilder::new("ZSCAN")
            .arg(key)
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
    }

    /// ZRANGEBYSCORE key min max — Return members by score range
    #[must_use = "call .build() to encode the command"]
    fn zrangebyscore<K: ToRedisArgs>(&self, key: K, min: f64, max: f64) -> CommandBuilder {
        CommandBuilder::new("ZRANGEBYSCORE")
            .arg(key)
            .arg(min)
            .arg(max)
    }

    /// ZRANGEBYSCORE key min max WITHSCORES — Return members with scores by score range
    #[must_use = "call .build() to encode the command"]
    fn zrangebyscore_withscores<K: ToRedisArgs>(
        &self,
        key: K,
        min: f64,
        max: f64,
    ) -> CommandBuilder {
        CommandBuilder::new("ZRANGEBYSCORE")
            .arg(key)
            .arg(min)
            .arg(max)
            .arg("WITHSCORES")
    }

    /// ZRANGEBYSCORE key min max LIMIT offset count — Range with pagination
    #[must_use = "call .build() to encode the command"]
    fn zrangebyscore_limit<K: ToRedisArgs>(
        &self,
        key: K,
        min: f64,
        max: f64,
        offset: i64,
        count: i64,
    ) -> CommandBuilder {
        CommandBuilder::new("ZRANGEBYSCORE")
            .arg(key)
            .arg(min)
            .arg(max)
            .arg("LIMIT")
            .arg(offset)
            .arg(count)
    }

    /// SUBSCRIBE channel [channel ...] — Subscribe to channels
    #[must_use = "call .build() to encode the command"]
    fn subscribe<K: ToRedisArgs>(&self, channels: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("SUBSCRIBE");
        for ch in channels {
            builder = builder.arg(ch);
        }
        builder
    }

    /// UNSUBSCRIBE — Unsubscribe from all channels
    #[must_use = "call .build() to encode the command"]
    fn unsubscribe(&self) -> CommandBuilder {
        CommandBuilder::new("UNSUBSCRIBE")
    }

    /// UNSUBSCRIBE channel [channel ...] — Unsubscribe from specific channels
    #[must_use = "call .build() to encode the command"]
    fn unsubscribe_channels<K: ToRedisArgs>(&self, channels: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("UNSUBSCRIBE");
        for ch in channels {
            builder = builder.arg(ch);
        }
        builder
    }

    /// PSUBSCRIBE pattern [pattern ...] — Subscribe by pattern
    #[must_use = "call .build() to encode the command"]
    fn psubscribe<K: ToRedisArgs>(&self, patterns: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("PSUBSCRIBE");
        for p in patterns {
            builder = builder.arg(p);
        }
        builder
    }

    /// PUNSUBSCRIBE — Unsubscribe from all patterns
    #[must_use = "call .build() to encode the command"]
    fn punsubscribe(&self) -> CommandBuilder {
        CommandBuilder::new("PUNSUBSCRIBE")
    }

    /// PUNSUBSCRIBE pattern [pattern ...] — Unsubscribe from specific patterns
    #[must_use = "call .build() to encode the command"]
    fn punsubscribe_patterns<K: ToRedisArgs>(&self, patterns: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("PUNSUBSCRIBE");
        for p in patterns {
            builder = builder.arg(p);
        }
        builder
    }

    /// MULTI — Start a transaction
    #[must_use = "call .build() to encode the command"]
    fn multi(&self) -> CommandBuilder {
        CommandBuilder::new("MULTI")
    }

    /// EXEC — Execute the transaction
    #[must_use = "call .build() to encode the command"]
    fn exec(&self) -> CommandBuilder {
        CommandBuilder::new("EXEC")
    }

    /// DISCARD — Abort the transaction
    #[must_use = "call .build() to encode the command"]
    fn discard(&self) -> CommandBuilder {
        CommandBuilder::new("DISCARD")
    }

    /// WATCH key [key ...] — Monitor keys for transactional changes
    #[must_use = "call .build() to encode the command"]
    fn watch<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("WATCH");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }

    /// UNWATCH — Clear all watched keys
    #[must_use = "call .build() to encode the command"]
    fn unwatch(&self) -> CommandBuilder {
        CommandBuilder::new("UNWATCH")
    }

    /// SELECT index — Select database
    #[must_use = "call .build() to encode the command"]
    fn select(&self, index: i64) -> CommandBuilder {
        CommandBuilder::new("SELECT").arg(index)
    }

    /// TYPE key — Get the type of a key
    #[must_use = "call .build() to encode the command"]
    fn type_(&self, key: impl ToRedisArgs) -> CommandBuilder {
        CommandBuilder::new("TYPE").arg(key)
    }

    /// MOVE key db — Move a key to another database
    #[must_use = "call .build() to encode the command"]
    fn move_key<K: ToRedisArgs>(&self, key: K, db: i64) -> CommandBuilder {
        CommandBuilder::new("MOVE").arg(key).arg(db)
    }

    /// RENAME key newkey — Rename a key
    #[must_use = "call .build() to encode the command"]
    fn rename<K: ToRedisArgs>(&self, key: K, newkey: K) -> CommandBuilder {
        CommandBuilder::new("RENAME").arg(key).arg(newkey)
    }

    /// RENAMENX key newkey — Rename a key only if it doesn't exist
    #[must_use = "call .build() to encode the command"]
    fn renamemx<K: ToRedisArgs>(&self, key: K, newkey: K) -> CommandBuilder {
        CommandBuilder::new("RENAMENX").arg(key).arg(newkey)
    }

    /// SORT key — Sort a list/set/zset
    #[must_use = "call .build() to encode the command"]
    fn sort<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("SORT").arg(key)
    }

    /// SORT key LIMIT offset count — Sort with pagination
    #[must_use = "call .build() to encode the command"]
    fn sort_limit<K: ToRedisArgs>(&self, key: K, offset: i64, count: i64) -> CommandBuilder {
        CommandBuilder::new("SORT")
            .arg(key)
            .arg("LIMIT")
            .arg(offset)
            .arg(count)
    }

    /// SORT key LIMIT offset count ORDER — Sort with pagination and ordering
    #[must_use = "call .build() to encode the command"]
    fn sort_limit_order<K: ToRedisArgs>(
        &self,
        key: K,
        offset: i64,
        count: i64,
        order: &str,
    ) -> CommandBuilder {
        CommandBuilder::new("SORT")
            .arg(key)
            .arg("LIMIT")
            .arg(offset)
            .arg(count)
            .arg(order)
    }

    /// SCAN cursor — Incrementally iterate keyspace
    #[must_use = "call .build() to encode the command"]
    fn scan(&self, cursor: i64) -> CommandBuilder {
        CommandBuilder::new("SCAN").arg(cursor)
    }

    /// SCAN cursor MATCH pattern [COUNT count] — Incrementally iterate with pattern matching
    #[must_use = "call .build() to encode the command"]
    fn scan_match(&self, cursor: i64, pattern: &str) -> CommandBuilder {
        CommandBuilder::new("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
    }

    /// TOUCH key [key ...] — Update access time of keys
    #[must_use = "call .build() to encode the command"]
    fn touch<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("TOUCH");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }

    /// SAVE — Synchronously save to disk
    #[must_use = "call .build() to encode the command"]
    fn save(&self) -> CommandBuilder {
        CommandBuilder::new("SAVE")
    }

    /// BGSAVE — Asynchronously save to disk
    #[must_use = "call .build() to encode the command"]
    fn bgsave(&self) -> CommandBuilder {
        CommandBuilder::new("BGSAVE")
    }

    /// FLUSHALL — Delete all keys from all databases
    #[must_use = "call .build() to encode the command"]
    fn flushall(&self) -> CommandBuilder {
        CommandBuilder::new("FLUSHALL")
    }

    /// PTTL key — Get time to live in milliseconds
    #[must_use = "call .build() to encode the command"]
    fn pttl<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("PTTL").arg(key)
    }

    /// PEXPIRE key milliseconds — Set expiry in milliseconds
    #[must_use = "call .build() to encode the command"]
    fn pexpire<K: ToRedisArgs>(&self, key: K, ms: i64) -> CommandBuilder {
        CommandBuilder::new("PEXPIRE").arg(key).arg(ms)
    }

    /// PEXPIREAT key timestamp-ms — Set expiry at unix time in milliseconds
    #[must_use = "call .build() to encode the command"]
    fn pexpireat<K: ToRedisArgs>(&self, key: K, timestamp_ms: i64) -> CommandBuilder {
        CommandBuilder::new("PEXPIREAT").arg(key).arg(timestamp_ms)
    }

    /// PERSIST key — Remove the existing timeout on a key
    #[must_use = "call .build() to encode the command"]
    fn persist<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("PERSIST").arg(key)
    }

    /// SHUTDOWN — Synchronously shut down the server
    #[must_use = "call .build() to encode the command"]
    fn shutdown(&self) -> CommandBuilder {
        CommandBuilder::new("SHUTDOWN")
    }

    /// SHUTDOWN NOSAVE — Shut down without saving
    #[must_use = "call .build() to encode the command"]
    fn shutdown_nosave(&self) -> CommandBuilder {
        CommandBuilder::new("SHUTDOWN").arg("NOSAVE")
    }

    /// INFO — Get server information
    #[must_use = "call .build() to encode the command"]
    fn info(&self) -> CommandBuilder {
        CommandBuilder::new("INFO")
    }

    /// INFO server — Get server section information
    #[must_use = "call .build() to encode the command"]
    fn info_section(&self, section: &str) -> CommandBuilder {
        CommandBuilder::new("INFO").arg(section)
    }

    /// CONFIG GET parameter — Get a config parameter
    #[must_use = "call .build() to encode the command"]
    fn config_get(&self, parameter: &str) -> CommandBuilder {
        CommandBuilder::new("CONFIG").arg("GET").arg(parameter)
    }
}

// Blanket impl so () implements Commands
impl Commands for () {}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::Commands;
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
            buf.unwrap().as_ref(),
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n"
        );
    }

    #[test]
    fn test_command_set_ex_encoding() {
        let buf = ().set_ex("key", "val", 60).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nEXPIRE\r\n$3\r\nkey\r\n$2\r\n60\r\n"
        );
    }

    #[test]
    fn test_command_publish_encoding() {
        let buf = ().publish("channel", "message").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nHSET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n$6\r\nvalue1\r\n"
        );
    }

    #[test]
    fn test_command_hget_encoding() {
        let buf = ().hget("myhash", "field1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nHGET\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
        );
    }

    #[test]
    fn test_command_sadd_encoding() {
        let buf = ().sadd("myset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nSADD\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_sismember_encoding() {
        let buf = ().sismember("myset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$9\r\nSISMEMBER\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_srem_encoding() {
        let buf = ().srem("myset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nSREM\r\n$5\r\nmyset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_setex_encoding() {
        let buf = ().setex("key", 60, "val").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nSETEX\r\n$3\r\nkey\r\n$2\r\n60\r\n$3\r\nval\r\n"
        );
    }

    #[test]
    fn test_command_incrby_encoding() {
        let buf = ().incrby("counter", 5).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nINCRBY\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
        );
    }

    #[test]
    fn test_command_append_encoding() {
        let buf = ().append("key", "hello").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nDECRBY\r\n$7\r\ncounter\r\n$1\r\n5\r\n"
        );
    }

    #[test]
    fn test_command_setnx_encoding() {
        let buf = ().setnx("key", "value").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$5\r\nSETNX\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[test]
    fn test_command_mget_encoding() {
        let buf = <() as Commands>::mget(&(), &["key1", "key2"])
            .build()
            .unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nMGET\r\n$4\r\nkey1\r\n$4\r\nkey2\r\n"
        );
    }

    #[test]
    fn test_command_mset_encoding() {
        let buf = <() as Commands>::mset(&(), &[("key1", "val1"), ("key2", "val2")])
            .build()
            .unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*5\r\n$4\r\nMSET\r\n$4\r\nkey1\r\n$4\r\nval1\r\n$4\r\nkey2\r\n$4\r\nval2\r\n"
        );
    }

    #[test]
    fn test_command_msetnx_encoding() {
        let buf = <() as Commands>::msetnx(&(), &[("key1", "val1"), ("key2", "val2")])
            .build()
            .unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$8\r\nGETRANGE\r\n$3\r\nkey\r\n$1\r\n0\r\n$2\r\n-1\r\n"
        );
    }

    #[test]
    fn test_command_setrange_encoding() {
        let buf = ().setrange("key", 5, "value").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$8\r\nSETRANGE\r\n$3\r\nkey\r\n$1\r\n5\r\n$5\r\nvalue\r\n"
        );
    }

    #[test]
    fn test_command_setbit_encoding() {
        let buf = ().setbit("key", 0, 1).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$6\r\nSETBIT\r\n$3\r\nkey\r\n$1\r\n0\r\n$1\r\n1\r\n"
        );
    }

    #[test]
    fn test_command_getbit_encoding() {
        let buf = ().getbit("key", 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$8\r\nBITCOUNT\r\n$3\r\nkey\r\n$1\r\n0\r\n$2\r\n-1\r\n"
        );
    }

    #[test]
    fn test_command_hdel_encoding() {
        let buf = ().hdel("myhash", "field1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nHDEL\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
        );
    }

    #[test]
    fn test_command_hdel_fields_encoding() {
        let buf = ().hdel_fields("myhash", &["f1", "f2"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*6\r\n$5\r\nHMSET\r\n$6\r\nmyhash\r\n$2\r\nf1\r\n$2\r\nv1\r\n$2\r\nf2\r\n$2\r\nv2\r\n"
        );
    }

    #[test]
    fn test_command_hincrby_encoding() {
        let buf = ().hincrby("myhash", "counter", 5).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$7\r\nHEXISTS\r\n$6\r\nmyhash\r\n$6\r\nfield1\r\n"
        );
    }

    #[test]
    fn test_command_hscan_encoding() {
        let buf = ().hscan("myhash", 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$5\r\nHSCAN\r\n$6\r\nmyhash\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_hscan_match_encoding() {
        let buf = ().hscan_match("myhash", 0, "f*").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
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
        let buf = <() as Commands>::sinter(&(), &["set1", "set2"])
            .build()
            .unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nSINTER\r\n$4\r\nset1\r\n$4\r\nset2\r\n"
        );
    }

    #[test]
    fn test_command_sunion_encoding() {
        let buf = <() as Commands>::sunion(&(), &["set1", "set2"])
            .build()
            .unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nSUNION\r\n$4\r\nset1\r\n$4\r\nset2\r\n"
        );
    }

    #[test]
    fn test_command_smove_encoding() {
        let buf = ().smove("src", "dst", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nSMOVE\r\n$3\r\nsrc\r\n$3\r\ndst\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_sscan_encoding() {
        let buf = ().sscan("myset", 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$5\r\nSSCAN\r\n$5\r\nmyset\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_sscan_match_encoding() {
        let buf = ().sscan_match("myset", 0, "m*").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*5\r\n$5\r\nSSCAN\r\n$5\r\nmyset\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$2\r\nm*\r\n"
        );
    }

    #[test]
    fn test_command_lpush_encoding() {
        let buf = ().lpush("mylist", &["v1", "v2"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nLPUSH\r\n$6\r\nmylist\r\n$2\r\nv1\r\n$2\r\nv2\r\n"
        );
    }

    #[test]
    fn test_command_rpush_encoding() {
        let buf = ().rpush("mylist", &["v1", "v2"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$6\r\nLRANGE\r\n$6\r\nmylist\r\n$1\r\n0\r\n$2\r\n-1\r\n"
        );
    }

    #[test]
    fn test_command_lindex_encoding() {
        let buf = ().lindex("mylist", 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nLINDEX\r\n$6\r\nmylist\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_lset_encoding() {
        let buf = ().lset("mylist", 0, "v").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nLSET\r\n$6\r\nmylist\r\n$1\r\n0\r\n$1\r\nv\r\n"
        );
    }

    #[test]
    fn test_command_lrem_encoding() {
        let buf = ().lrem("mylist", 0, "v").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nLREM\r\n$6\r\nmylist\r\n$1\r\n0\r\n$1\r\nv\r\n"
        );
    }

    #[test]
    fn test_command_ltrim_encoding() {
        let buf = ().ltrim("mylist", 0, 10).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nLTRIM\r\n$6\r\nmylist\r\n$1\r\n0\r\n$2\r\n10\r\n"
        );
    }

    #[test]
    fn test_command_blpop_encoding() {
        let buf = ().blpop(&["list1", "list2"], 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nBLPOP\r\n$5\r\nlist1\r\n$5\r\nlist2\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_brpop_encoding() {
        let buf = ().brpop(&["list1", "list2"], 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$5\r\nBRPOP\r\n$5\r\nlist1\r\n$5\r\nlist2\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_zadd_encoding() {
        let buf = ().zadd("myzset", 1.0, "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nZADD\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_zadd_multi_encoding() {
        let buf = ().zadd_multi("myzset", &[(1.0, "m1"), (2.0, "m2")]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*6\r\n$4\r\nZADD\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$2\r\nm1\r\n$3\r\n2.0\r\n$2\r\nm2\r\n"
        );
    }

    #[test]
    fn test_command_zrem_encoding() {
        let buf = ().zrem("myzset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nZREM\r\n$6\r\nmyzset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_zrem_members_encoding() {
        let buf = ().zrem_members("myzset", &["m1", "m2"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nZREM\r\n$6\r\nmyzset\r\n$2\r\nm1\r\n$2\r\nm2\r\n"
        );
    }

    #[test]
    fn test_command_zrange_encoding() {
        let buf = ().zrange("myzset", 0, -1).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$6\r\nZRANGE\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$2\r\n-1\r\n"
        );
    }

    #[test]
    fn test_command_zrange_withscores_encoding() {
        let buf = ().zrange_withscores("myzset", 0, -1).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*5\r\n$6\r\nZRANGE\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$2\r\n-1\r\n$10\r\nWITHSCORES\r\n"
        );
    }

    #[test]
    fn test_command_zrank_encoding() {
        let buf = ().zrank("myzset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$5\r\nZRANK\r\n$6\r\nmyzset\r\n$7\r\nmember1\r\n"
        );
    }

    #[test]
    fn test_command_zscore_encoding() {
        let buf = ().zscore("myzset", "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$6\r\nZCOUNT\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n"
        );
    }

    #[test]
    fn test_command_zincrby_encoding() {
        let buf = ().zincrby("myzset", 5.0, "member1").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$7\r\nZPOPMIN\r\n$6\r\nmyzset\r\n$1\r\n3\r\n"
        );
    }

    #[test]
    fn test_command_zscan_encoding() {
        let buf = ().zscan("myzset", 0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$5\r\nZSCAN\r\n$6\r\nmyzset\r\n$1\r\n0\r\n"
        );
    }

    #[test]
    fn test_command_zscan_match_encoding() {
        let buf = ().zscan_match("myzset", 0, "m*").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*5\r\n$5\r\nZSCAN\r\n$6\r\nmyzset\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$2\r\nm*\r\n"
        );
    }

    #[test]
    fn test_command_zrangebyscore_encoding() {
        let buf = ().zrangebyscore("myzset", 1.0, 10.0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*4\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n"
        );
    }

    #[test]
    fn test_command_zrangebyscore_withscores_encoding() {
        let buf = ().zrangebyscore_withscores("myzset", 1.0, 10.0).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*5\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n$10\r\nWITHSCORES\r\n"
        );
    }

    #[test]
    fn test_command_zrangebyscore_limit_encoding() {
        let buf = ().zrangebyscore_limit("myzset", 1.0, 10.0, 0, 5).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*7\r\n$13\r\nZRANGEBYSCORE\r\n$6\r\nmyzset\r\n$3\r\n1.0\r\n$4\r\n10.0\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$1\r\n5\r\n"
        );
    }

    #[test]
    fn test_command_subscribe_encoding() {
        let buf = ().subscribe(&["ch1", "ch2"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$11\r\nUNSUBSCRIBE\r\n$3\r\nch1\r\n$3\r\nch2\r\n"
        );
    }

    #[test]
    fn test_command_psubscribe_encoding() {
        let buf = ().psubscribe(&["pattern*", "test?*"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$4\r\nMOVE\r\n$5\r\nmykey\r\n$1\r\n1\r\n"
        );
    }

    #[test]
    fn test_command_rename_encoding() {
        let buf = ().rename("mykey", "newkey").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nRENAME\r\n$5\r\nmykey\r\n$6\r\nnewkey\r\n"
        );
    }

    #[test]
    fn test_command_renamemx_encoding() {
        let buf = ().renamemx("mykey", "newkey").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*5\r\n$4\r\nSORT\r\n$6\r\nmylist\r\n$5\r\nLIMIT\r\n$1\r\n0\r\n$2\r\n10\r\n"
        );
    }

    #[test]
    fn test_command_sort_limit_order_encoding() {
        let buf = ().sort_limit_order("mylist", 0, 10, "DESC").build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*4\r\n$4\r\nSCAN\r\n$1\r\n0\r\n$5\r\nMATCH\r\n$4\r\nfoo*\r\n"
        );
    }

    #[test]
    fn test_command_touch_encoding() {
        let buf = ().touch(&["k1", "k2", "k3"]).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$7\r\nPEXPIRE\r\n$5\r\nmykey\r\n$5\r\n10000\r\n"
        );
    }

    #[test]
    fn test_command_pexpireat_encoding() {
        let buf = ().pexpireat("mykey", 1_609_459_200_000).build().unwrap();
        assert_eq!(
            buf.unwrap().as_ref(),
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
            buf.unwrap().as_ref(),
            b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$9\r\nmaxmemory\r\n"
        );
    }
}
