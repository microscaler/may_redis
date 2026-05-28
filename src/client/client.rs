// Client — RedisClient implementation
//
// Provides the main user-facing API for connecting to Redis and executing commands.

use crate::core::{FromRedisValue, RedisError, RedisValue, ToRedisArgs};
use crate::protocol::{builder::CommandBuilder, commands::Commands};
use may::go;
use may::sync::spsc;
use std::sync::Arc;
use std::time::Duration;

use crate::connection::{Connection, Request};

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

    /// Execute a command with a configurable timeout and return the typed result.
    ///
    /// # Arguments
    /// * `cmd` - The command to execute, built with [`CommandBuilder`]
    /// * `timeout` - Maximum duration to wait for a response
    ///
    /// # Returns
    /// The decoded response of type `T`, or a [`RedisError::Connection`] timeout error.
    ///
    /// # Timeout behavior
    /// Polls the response channel with `try_recv()` in a loop, racing against
    /// a timeout coroutine that signals on a separate spsc channel. If the
    /// response arrives first it is returned immediately; if the timeout fires
    /// first a `Connection` error is returned.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] if the TCP connection fails, the
    /// response channel is closed, or the timeout expires before a response
    /// is received.
    pub fn execute_with_timeout<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
        timeout: Duration,
    ) -> Result<T, RedisError> {
        // Build the command into RESP bytes
        let data = cmd.build();

        // Create a channel for this request's response
        let (tx, rx) = spsc::channel();

        // Create and send the request
        let request = Request::new(data.to_vec(), tx);
        let _tag = self.inner.connection.send(request);

        // Spsc channel for timeout signaling — keeps `rx` on the main thread.
        let (timeout_tx, timeout_rx) = spsc::channel::<()>();

        // Spawn a timeout coroutine that signals via the separate channel.
        go!(move || {
            std::thread::sleep(timeout);
            let _ = timeout_tx.send(());
        });

        // Poll loop: wait for response or timeout signal.
        // `rx.try_recv()` is non-blocking so the main coroutine yields
        // and lets the connection-loop epoll coroutine run.
        let response = loop {
            // Try to receive the response first.
            if let Ok(resp) = rx.try_recv() {
                break resp;
            }
            // Check for timeout.
            if timeout_rx.try_recv().is_ok() {
                return Err(RedisError::Connection(format!(
                    "command execution timed out after {timeout:?}"
                )));
            }
            // Yield to let the connection loop make progress.
            may::coroutine::yield_now();
        };

        // Check for Redis protocol errors before type conversion.
        // This preserves the original Redis error message instead of
        // wrapping it in a generic Parse error.
        if let RedisValue::Error(msg) = response {
            return Err(RedisError::Protocol(msg));
        }

        // Convert RedisValue to the requested type
        T::from_redis_value(&response)
    }

    /// Execute a command with a timeout in seconds and return the typed result.
    ///
    /// # Arguments
    /// * `cmd` - The command to execute, built with [`CommandBuilder`]
    /// * `seconds` - Maximum seconds to wait for a response
    ///
    /// # Returns
    /// The decoded response of type `T`, or a [`RedisError::Connection`] timeout error.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] if the TCP connection fails, the
    /// response channel is closed, or the timeout expires before a response
    /// is received.
    pub fn execute_timeout<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
        seconds: u32,
    ) -> Result<T, RedisError> {
        self.execute_with_timeout(cmd, Duration::from_secs(u64::from(seconds)))
    }

    /// Execute a command and return the typed result (30-second default timeout).
    ///
    /// # Arguments
    /// * `cmd` - The command to execute, built with [`CommandBuilder`]
    ///
    /// # Returns
    /// The decoded response of type `T`, or a [`RedisError`] on failure.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] if the TCP connection fails, the
    /// response channel is closed, or the timeout expires before a response
    /// is received. Returns [`RedisError::Parse`] if the response cannot be
    /// converted to the requested type.
    pub fn execute<T: FromRedisValue>(&self, cmd: CommandBuilder) -> Result<T, RedisError> {
        self.execute_with_timeout(cmd, Duration::from_secs(30))
    }

    /// Send a PING command and return the response.
    ///
    /// # Returns
    /// `Ok(String)` with "PONG" on success, or a [`RedisError`] on failure.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the server responds with anything other
    /// than "PONG", or if the connection fails.
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
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
    #[ignore = "requires live Redis server"]
    fn test_integration_ping() {
        run_may(|| {
            let client = shared_client();
            let result = client.ping();
            assert_eq!(result.unwrap(), "PONG");
            client.execute::<String>(client.flushdb()).ok();
        });
    }

    #[test]
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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
    #[ignore = "requires live Redis server"]
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

    // -----------------------------------------------------------------------
    // Concurrency tests (Epic 6.2)
    // -----------------------------------------------------------------------

    /// Test that 3 coroutines can each send GET for different keys
    /// and all receive correct responses.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_concurrent_requests() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Pre-populate data
            client
                .execute::<()>(client.set("concurrent:x", "alpha"))
                .unwrap();
            client
                .execute::<()>(client.set("concurrent:y", "beta"))
                .unwrap();
            client
                .execute::<()>(client.set("concurrent:z", "gamma"))
                .unwrap();

            // Clone client and use both copies — tests Send + Sync
            let c1 = client.clone();
            let c2 = client.clone();
            let c3 = client.clone();

            let v1: Option<String> = c1.execute(c1.get("concurrent:x")).unwrap();
            let v2: Option<String> = c2.execute(c2.get("concurrent:y")).unwrap();
            let v3: Option<String> = c3.execute(c3.get("concurrent:z")).unwrap();

            assert_eq!(v1, Some("alpha".to_string()));
            assert_eq!(v2, Some("beta".to_string()));
            assert_eq!(v3, Some("gamma".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test that a pipeline and single commands can interleave without
    /// cross-talk.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_pipeline_concurrent() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client.execute::<()>(client.set("pc:a", "1")).unwrap();
            client.execute::<()>(client.set("pc:b", "2")).unwrap();

            let c1 = client.clone();
            let c2 = client.clone();

            // Pipeline on c1
            let mut pipe = c1.pipeline();
            pipe.add(c1.set("pc:p1", "pp1"));
            pipe.add(c1.set("pc:p2", "pp2"));
            pipe.add(c1.get("pc:p1"));
            let ((), (), got_p1): ((), (), Option<String>) = pipe.execute().unwrap();

            // Single commands on c2
            let v_a: Option<String> = c2.execute(c2.get("pc:a")).unwrap();
            let v_b: Option<String> = c2.execute(c2.get("pc:b")).unwrap();

            assert_eq!(got_p1, Some("pp1".to_string()));
            assert_eq!(v_a, Some("1".to_string()));
            assert_eq!(v_b, Some("2".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test two concurrent pipelines running simultaneously.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_concurrent_pipelines() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let c1 = client.clone();
            let c2 = client.clone();

            // Pipeline 1: set p1a, p1b, get p1a
            let mut pipe1 = c1.pipeline();
            pipe1.add(c1.set("cp1:a", "val1a"));
            pipe1.add(c1.set("cp1:b", "val1b"));
            pipe1.add(c1.get("cp1:a"));
            let ((), (), got_a1): ((), (), Option<String>) = pipe1.execute().unwrap();

            // Pipeline 2: set p2a, p2b, get p2b
            let mut pipe2 = c2.pipeline();
            pipe2.add(c2.set("cp2:a", "val2a"));
            pipe2.add(c2.set("cp2:b", "val2b"));
            pipe2.add(c2.get("cp2:b"));
            let ((), (), got_p2_b): ((), (), Option<String>) = pipe2.execute().unwrap();

            assert_eq!(got_a1, Some("val1a".to_string()));
            assert_eq!(got_p2_b, Some("val2b".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test that monotonically increasing tags are unique across many
    /// requests (proves AtomicUsize ordering).
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_request_ordering() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Send 50 commands in a pipeline — tags must be unique
            let mut pipe = client.pipeline();
            for i in 0..50 {
                pipe.add(client.set(format!("order:{i}"), i.to_string()));
            }
            let results: Vec<()> = pipe.execute().unwrap();
            assert_eq!(results.len(), 50);

            // Verify all values were set
            let mut pipe = client.pipeline();
            for i in 0..50 {
                pipe.add(client.get(format!("order:{i}")));
            }
            let results: Vec<Option<String>> = pipe.execute().unwrap();
            for (i, val) in results.iter().enumerate() {
                assert_eq!(val, &Some(i.to_string()), "order:{i} mismatch");
            }

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test that responses are dispatched to the correct channels.
    /// Send 10 commands from 10 coroutines (cloned clients), verify
    /// each gets its own response.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_response_correlation() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Pre-populate 10 distinct keys
            for i in 0..10 {
                client
                    .execute::<()>(client.set(format!("rc:{i}"), format!("resp-{i}")))
                    .unwrap();
            }

            // Use 10 clones to verify each response goes to the right channel
            for i in 0..10 {
                let c = client.clone();
                let expected = format!("resp-{i}");
                let v: Option<String> = c.execute(c.get(format!("rc:{i}"))).unwrap();
                assert_eq!(v, Some(expected), "correlation failed for key {i}");
            }

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    // -----------------------------------------------------------------------
    // Error handling tests (Epic 6.3)
    // -----------------------------------------------------------------------

    /// Test that a server error message propagates as RedisError.
    /// Redis returns "ERR WRONG_TYPE..." for INCR on a string value.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_server_error_propagation() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Set a non-integer value
            client
                .execute::<()>(client.set("err:str", "not_a_number"))
                .unwrap();

            // INCR on a string value should return a server error
            let result: Result<i64, _> = client.execute(client.incr("err:str"));
            assert!(
                result.is_err(),
                "INCR on string should error, got: {result:?}"
            );
            let err = result.unwrap_err();
            assert!(
                format!("{err}").to_lowercase().contains("err")
                    || format!("{err}").to_lowercase().contains("integer"),
                "error should mention 'ERR' or 'integer', got: {err}"
            );

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test wrong-type FromRedisValue error: response is Integer but
    /// caller expects String.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_wrong_type_extraction() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // DBSIZE returns an Integer, but we try to extract as String
            let result: Result<String, _> = client.execute(client.dbsize());
            assert!(result.is_err(), "DBSIZE→String should error");
            let err = result.unwrap_err();
            assert!(
                format!("{err}").to_lowercase().contains("parse")
                    || format!("{err}").to_lowercase().contains("integer"),
                "error should be a Parse error, got: {err}"
            );

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test empty pipeline handling — pipeline with no commands added
    /// should not panic and should return an appropriate error or empty result.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_empty_pipeline() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let mut pipe = client.pipeline();
            // No commands added — execute an empty pipeline
            let result: Result<Vec<()>, _> = pipe.execute();
            // Empty pipeline should return Ok(vec![]) — no commands sent, no responses to collect
            assert_eq!(
                result.unwrap(),
                Vec::<()>::new(),
                "empty pipeline should return empty vec"
            );

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test Null response handling: GET a missing key returns Null,
    /// which FromRedisValue converts to None.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_null_response_handling() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            let result: Result<Option<String>, _> = client.execute(client.get("missing_key"));
            assert_eq!(result.unwrap(), None, "GET missing key should return None");

            // Test that existing key returns Some
            client
                .execute::<()>(client.set("null_test:exists", "val"))
                .unwrap();
            let result: Result<Option<String>, _> = client.execute(client.get("null_test:exists"));
            assert_eq!(result.unwrap(), Some("val".to_string()));

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test that Redis errors from the server (e.g., WRONGTYPE) propagate
    /// as a parse error when FromRedisValue can't convert.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_redis_server_error_value() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Set a string
            client
                .execute::<()>(client.set("srv:str", "hello"))
                .unwrap();

            // Try to get it as an Integer — Redis will return the string value,
            // FromRedisValue for i64 will reject it
            let result: Result<i64, _> = client.execute(client.get("srv:str"));
            assert!(
                result.is_err(),
                "GET string→i64 should error, got: {result:?}"
            );
            let err = result.unwrap_err();
            assert!(
                format!("{err}").to_lowercase().contains("parse")
                    || format!("{err}").to_lowercase().contains("expected"),
                "error should be a parse error, got: {err}"
            );

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test that pipeline error handling: a failing command in a
    /// pipeline returns the server error response.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_pipeline_error_handling() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            // Set a string value
            client
                .execute::<()>(client.set("pipe_err:str", "hello"))
                .unwrap();

            // Pipeline: try INCR on string (will error) then GET it
            let mut pipe = client.pipeline();
            pipe.add(client.incr("pipe_err:str")); // This will error
            pipe.add(client.get("pipe_err:str"));

            // First element gets the error response as a RedisValue::Error,
            // which i64::from_redis_value cannot convert.
            let result: Result<(i64, Option<String>), _> = pipe.execute();
            assert!(
                result.is_err(),
                "pipeline with failing command should error: {result:?}"
            );

            client.execute::<()>(client.flushdb()).ok();
        });
    }

    /// Test error message content for INCR on string — should include
    /// a descriptive error.
    #[test]
    #[ignore = "requires live Redis server"]
    fn test_integration_incr_string_error_message() {
        run_may(|| {
            let client = shared_client();
            client.execute::<()>(client.flushdb()).ok();

            client
                .execute::<()>(client.set("msg_err", "not_num"))
                .unwrap();

            let result: Result<i64, _> = client.execute(client.incr("msg_err"));
            assert!(result.is_err());
            let err = result.unwrap_err();
            let msg = format!("{err}");
            // The error should be descriptive
            assert!(!msg.is_empty(), "error message should not be empty");

            client.execute::<()>(client.flushdb()).ok();
        });
    }
}
