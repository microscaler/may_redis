// Commands — admin trait
//
// Provides all Admin commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Admin command methods.
pub trait AdminCommands: Sized {
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
}
