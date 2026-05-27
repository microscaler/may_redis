// Client — RedisClient implementation
//
// Provides the main user-facing API for connecting to Redis and executing commands.

use crate::core::{FromRedisValue, RedisError, ToRedisArgs};
use crate::protocol::{builder::CommandBuilder, commands::Commands};
use std::sync::Arc;

use crate::connection::{Connection, Request};
use may::sync::spsc;

use super::pipeline::Pipeline;

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
    pub fn connect(host: &str, port: u16) -> Result<Self, crate::connection::ConnectionError> {
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

    /// Create a pipeline for batch command execution.
    #[must_use]
    pub fn pipeline(&self) -> Pipeline<'_> {
        Pipeline::new(&self.inner.connection)
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
#[allow(clippy::used_underscore_items)]
mod tests {
    use super::*;
    use may::config;
    use may::go;
    use std::sync::Mutex;
    use std::sync::Once;

    /// One-time initialization of the may coroutine runtime.
    ///
    /// The may scheduler is lazily initialized on first call to
    /// `config().set_workers()`. We initialize it once so that every
    /// test thread has a valid may context before spawning coroutines.
    ///
    /// Without this, `go!` panics on fresh std threads (e.g. CI runners)
    /// because the may scheduler hasn't been started yet.
    fn init_may_runtime() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            config().set_workers(1);
        });
    }

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

    // ---------------------------------------------------------------------------
    // Integration tests — require a Redis server on localhost:6379
    // ---------------------------------------------------------------------------
    // Each test runs inside `run_may()` which spawns the test body as a
    // coroutine via `go!` (the may crate's coroutine spawning macro). This
    // ensures the may scheduler is properly initialized on the current thread
    // before spawning any coroutines.
    //
    // CRITICAL: We reuse a SINGLE shared RedisClient across all integration
    // tests. Creating a new connection per test spawns a new epoll coroutine
    // that gets cancelled on drop, exhausting the may scheduler's coroutine
    // pool after ~4 tests. Keeping one connection alive avoids this.
    // ---------------------------------------------------------------------------

    /// Returns the shared RedisClient, initializing it on first call.
    fn shared_client() -> RedisClient {
        static INIT: std::sync::Once = std::sync::Once::new();
        static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
        INIT.call_once(|| {
            CLIENT
                .set(
                    RedisClient::connect("127.0.0.1", 6379)
                        .expect("Redis must be running on localhost:6379"),
                )
                .ok();
        });
        CLIENT.get().expect("client not initialized").clone()
    }

    /// Run e2e test logic inside the may scheduler.
    ///
    /// Uses `go!` to spawn the test body as a may coroutine, then joins it.
    /// The coroutine's `rx.recv()` calls cooperatively yield, letting the
    /// connection-loop coroutine run and dispatch responses.
    ///
    /// `init_may_runtime()` must be called before `go!` to ensure the may
    /// scheduler is initialized. Without this, spawning coroutines on a fresh
    /// std thread (as happens in CI) will panic.
    fn run_may<F, T>(f: F) -> T
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        // Ensure the may scheduler is initialized on this thread before spawning
        // any coroutines. Without this, go! panics on fresh std threads.
        init_may_runtime();

        let wrapper = Arc::new(Mutex::new(None::<T>));
        let wrapper2 = Arc::clone(&wrapper);

        // `go!` spawns a coroutine and returns JoinHandle<()>.
        // The test value is stored in the wrapper; we extract it after join.
        let handle = go!(move || {
            let val = f();
            *wrapper2.lock().unwrap() = Some(val);
        });

        // JoinHandle::join() blocks at the coroutine level (park/unpark),
        // so it cooperatively yields while the scheduler runs the connection loop.
        let result = handle.join();
        match result {
            Ok(()) => wrapper
                .lock()
                .unwrap()
                .take()
                .expect("test coroutine did not store result"),
            Err(e) => panic!("test coroutine panicked: {e:?}"),
        }
    }

    #[test]
    fn test_integration_ping() {
        run_may(|| {
            let client = shared_client();
            let result = client.ping();
            assert_eq!(result.unwrap(), "PONG");
            client.execute::<String>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_set_get() {
        run_may(|| {
            let client = shared_client();
            client
                .execute::<()>(client.set("test_key", "hello"))
                .unwrap();
            let result: Option<String> = client.execute(client.get("test_key")).unwrap();
            assert_eq!(result, Some("hello".to_string()));
            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_incr() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let val: i64 = client.execute(client.incr("counter")).unwrap();
            assert_eq!(val, 1);

            let val: i64 = client.execute(client.incr("counter")).unwrap();
            assert_eq!(val, 2);

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_exists_del() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client.execute::<()>(client.set("key1", "val1")).unwrap();
            let exists: bool = client.execute(client.exists("key1")).unwrap();
            assert!(exists);

            let exists: bool = client.execute(client.exists("missing")).unwrap();
            assert!(!exists);

            client.execute::<()>(client.del("key1")).unwrap();
            let exists: bool = client.execute(client.exists("key1")).unwrap();
            assert!(!exists);

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_dbsize() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let size: usize = client.execute(client.dbsize()).unwrap();
            assert_eq!(size, 0);

            client.execute::<()>(client.set("a", "1")).unwrap();
            client.execute::<()>(client.set("b", "2")).unwrap();
            let size: usize = client.execute(client.dbsize()).unwrap();
            assert_eq!(size, 2);

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_set_ex_ttl() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client
                .execute::<()>(client.set_ex("ttl_key", "val", 60))
                .unwrap();
            let result: Option<String> = client.execute(client.get("ttl_key")).unwrap();
            assert_eq!(result, Some("val".to_string()));

            let ttl: i64 = client.execute(client.ttl("ttl_key")).unwrap();
            assert!(ttl > 0 && ttl <= 60);

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_keys() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client.execute::<()>(client.set("user:1", "alice")).unwrap();
            client.execute::<()>(client.set("user:2", "bob")).unwrap();
            client.execute::<()>(client.set("other:1", "x")).unwrap();

            let keys: Vec<String> = client.execute(client.keys("user:*")).unwrap();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains(&"user:1".to_string()));
            assert!(keys.contains(&"user:2".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_send_sync_clone() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let cloned = client.clone();
            cloned
                .execute::<()>(cloned.set("clone_test", "works"))
                .unwrap();
            let val: Option<String> = client.execute(cloned.get("clone_test")).unwrap();
            assert_eq!(val, Some("works".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_error_propagation() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client
                .execute::<()>(client.set("str_key", "not_a_number"))
                .unwrap();
            let result: Result<i64, _> = client.execute(client.incr("str_key"));
            assert!(result.is_err());

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_pipeline() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Build a pipeline with SET, SET, SET, GET
            let mut pipeline = client.pipeline();
            pipeline.add(client.set("pip:1", "a"));
            pipeline.add(client.set("pip:2", "b"));
            pipeline.add(client.set("pip:3", "c"));
            pipeline.add(client.get("pip:1"));

            let results: ((), (), (), Option<String>) = pipeline.execute().unwrap();
            assert_eq!(results, ((), (), (), Some("a".to_string())));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    #[test]
    fn test_integration_concurrent() {
        // Test that the shared client can be cloned and used from multiple
        // places. The may runtime handles coroutine yielding for I/O so
        // we verify the client is properly shareable via clone().
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Clone the client and use both copies — tests Send + Sync
            let c1 = client.clone();
            let c2 = client.clone();

            c1.execute::<()>(c1.set("concurrent:a", "1")).unwrap();
            c2.execute::<()>(c2.set("concurrent:b", "2")).unwrap();

            let v1: Option<String> = c1.execute(c1.get("concurrent:a")).unwrap();
            let v2: Option<String> = c2.execute(c2.get("concurrent:b")).unwrap();

            assert_eq!(v1, Some("1".to_string()));
            assert_eq!(v2, Some("2".to_string()));

            // Verify with KEYS
            let keys: Vec<String> = client.execute(client.keys("concurrent:*")).unwrap();
            assert_eq!(keys.len(), 2);

            client.execute::<()>(client.flushdb()).ok();
        });
    }
}
