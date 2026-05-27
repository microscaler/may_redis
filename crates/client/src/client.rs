// Client — RedisClient implementation
//
// Provides the main user-facing API for connecting to Redis and executing commands.

use base::{FromRedisValue, RedisError, ToRedisArgs};
use protocol::{builder::CommandBuilder, commands::Commands};
use std::sync::Arc;

use connection::{Connection, Request};
use may::sync::spsc;

/// Internal client state shared across coroutines.
struct InnerClient {
    connection: Connection,
}

/// Main entry point for Redis operations.
///
/// `RedisClient` is wrapped in `Arc<InnerClient>` so multiple coroutines
/// can share the same connection. It implements the [`Commands`] trait
/// for a familiar API surface.
#[derive(Clone)]
pub struct RedisClient {
    inner: Arc<InnerClient>,
}

impl RedisClient {
    /// Connect to a Redis server given a host and port.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    ///
    /// # Errors
    /// Returns [`ConnectionError`](connection::ConnectionError) if TCP connection fails.
    pub fn connect(host: &str, port: u16) -> Result<Self, connection::ConnectionError> {
        let connection = Connection::connect(host, port)?;
        Ok(Self {
            inner: Arc::new(InnerClient { connection }),
        })
    }

    /// Connect to a Redis server given a URL in the format `redis://host:port`.
    ///
    /// # Arguments
    /// * `url` - Connection URL (e.g., `redis://localhost:6379`)
    ///
    /// # Errors
    /// Returns [`RedisError`] if URL parsing fails, or [`ConnectionError`] if TCP connection fails.
    pub fn connect_url(url: &str) -> Result<Self, RedisError> {
        let url = url.strip_prefix("redis://").unwrap_or(url);
        let (host, port) = url.rsplit_once(':').ok_or_else(|| {
            RedisError::Parse("invalid URL format, expected redis://host:port".into())
        })?;
        let port: u16 = port
            .parse()
            .map_err(|e| RedisError::Parse(format!("invalid port: {e}")))?;
        Self::connect(host, port).map_err(|e| RedisError::Parse(format!("connection failed: {e}")))
    }

    /// Execute a command and return the typed result.
    ///
    /// # Arguments
    /// * `cmd` - The command to execute, built with [`CommandBuilder`]
    ///
    /// # Returns
    /// The decoded response of type `T`, or a [`RedisError`] on failure.
    pub fn execute<T: FromRedisValue>(&self, cmd: CommandBuilder) -> Result<T, RedisError> {
        // Build the command into RESP bytes
        let data = cmd.build();

        // Create a channel for this request's response
        let (tx, rx) = spsc::channel();

        // Create and send the request
        let request = Request::new(data.to_vec(), tx);
        let _tag = self.inner.connection.send(request);

        // Wait for response from the connection loop
        let response = rx
            .recv()
            .map_err(|_| RedisError::Parse("response channel closed".into()))?;

        // Convert RedisValue to the requested type
        T::from_redis_value(&response)
    }

    /// Send a PING command and return the response.
    ///
    /// # Returns
    /// `Ok(String)` with "PONG" on success, or a [`RedisError`] on failure.
    pub fn ping(&self) -> Result<String, RedisError> {
        let cmd = CommandBuilder::new("PING");
        let response = self.execute::<String>(cmd)?;
        if response == "PONG" {
            Ok(response)
        } else {
            Err(RedisError::Parse(format!(
                "unexpected PING response: {response}"
            )))
        }
    }
}

/// Implement the `Commands` trait on `RedisClient`.
///
/// Each method builds a `CommandBuilder` just like the default impl,
/// so the caller can either:
/// - Use the builder directly: `client.get("key").build()` for raw bytes
/// - Use typed execute: `client.execute(client.get("key"))` for typed results
impl Commands for RedisClient {
    #[allow(clippy::needless_pass_by_value)]
    fn get<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("GET").arg(key)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> CommandBuilder {
        CommandBuilder::new("SET").arg(key).arg(value)
    }

    #[allow(clippy::needless_pass_by_value)]
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

    #[allow(clippy::needless_pass_by_value)]
    fn exists<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("EXISTS").arg(key)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn del<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("DEL").arg(key)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn incr<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("INCR").arg(key)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn ttl<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
        CommandBuilder::new("TTL").arg(key)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn expire<K: ToRedisArgs>(&self, key: K, seconds: u32) -> CommandBuilder {
        CommandBuilder::new("EXPIRE").arg(key).arg(seconds)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn publish<K: ToRedisArgs, M: ToRedisArgs>(&self, channel: K, message: M) -> CommandBuilder {
        CommandBuilder::new("PUBLISH").arg(channel).arg(message)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn keys<K: ToRedisArgs>(&self, pattern: K) -> CommandBuilder {
        CommandBuilder::new("KEYS").arg(pattern)
    }

    fn dbsize(&self) -> CommandBuilder {
        CommandBuilder::new("DBSIZE")
    }

    fn flushdb(&self) -> CommandBuilder {
        CommandBuilder::new("FLUSHDB")
    }

    fn ping(&self) -> CommandBuilder {
        CommandBuilder::new("PING")
    }

    #[allow(clippy::needless_pass_by_value)]
    fn auth(&self, password: &str) -> CommandBuilder {
        CommandBuilder::new("AUTH").arg(password)
    }
}

/// Note: `Commands` is impl'd on `RedisClient` only.
/// `&RedisClient` gets it automatically via auto-deref — no separate impl needed.
/// The only exception is `ping`: the inherent `ping()` returns `Result<String, RedisError>`
/// (executes the command), while `Commands::ping()` returns `CommandBuilder` (builds it).
/// Auto-deref resolves `&RedisClient::ping()` to the *inherent* method, which is the
/// expected behavior — callers wanting the raw builder use `Commands::ping()`.

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that RedisClient struct is constructible
    #[test]
    fn test_redis_client_struct() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<RedisClient>();
    }

    /// Test that Commands trait methods are callable
    #[test]
    fn test_commands_trait_methods_exist() {
        fn _require_commands<T: Commands>() {}
        _require_commands::<RedisClient>();
    }
}
