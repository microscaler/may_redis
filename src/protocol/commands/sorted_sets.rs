// Commands — sorted_sets trait
//
// Provides all Sorted_Sets commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Sorted_Sets command methods.
pub trait SortedSetsCommands: Sized {

    /// ZREM key member — Remove a member from a sorted set
    #[must_use = "call .build() to encode the command"]
    fn zrem<K: ToRedisArgs, M: ToRedisArgs>(&self, key: K, member: M) -> CommandBuilder {
        CommandBuilder::new("ZREM").arg(key).arg(member)
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

}
