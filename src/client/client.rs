// RedisClient — main entry point for Redis operations.
//
// Provides RedisClient struct with connect(), connect_with_timeout(),
// connect_with_ssrf_protection() methods. URL-based connection is
// delegated to `client_url::connect_url()`.

use std::sync::Arc;
use std::time::Duration;

use super::client_url;
use super::pipeline::Pipeline;
use crate::connection::{Connection, SsrfConfig};
use crate::core::{FromRedisValue, RedisError};
use crate::protocol::builder::CommandBuilder;
use crate::protocol::commands::{
    AdminCommands, HashesCommands, ListsCommands, PubsubCommands, SetsCommands, SortedSetsCommands,
    StringsCommands, TransactionsCommands,
};

/// Default timeout for `execute()` — 5 seconds.
///
/// Security rationale: a 30-second default allows slow commands
/// (KEYS *, large FLUSHDB) to execute on the server for half a
/// minute before the client gives up.  5 seconds is a reasonable
/// upper bound for typical Redis operations and matches the redis-rs
/// crate's default.
const DEFAULT_EXECUTE_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// Inner client state
// ---------------------------------------------------------------------------

/// Internal client state shared across coroutines — visible to sibling client modules.
pub(super) struct InnerClient {
    pub(super) connection: Connection,
    /// Default timeout for `execute()` — overrides hardcoded 30s.
    pub(super) default_timeout: Duration,
    /// Command policy enforced on every `execute()` call.
    pub(super) command_policy: crate::protocol::builder::CommandPolicy,
}

// ---------------------------------------------------------------------------
// RedisClient
// ---------------------------------------------------------------------------

/// Main entry point for Redis operations.
///
/// `RedisClient` is wrapped in `Arc<InnerClient>` so multiple coroutines
/// can share the same connection. It implements the [`Commands`] trait
/// for a familiar API surface.
#[derive(Clone)]
pub struct RedisClient {
    pub(super) inner: Arc<InnerClient>,
}

impl RedisClient {
    /// Connect to a Redis server given a host and port.
    ///
    /// Uses the default timeout of 5 seconds. See
    /// [`Self::connect_with_timeout`] for a custom timeout.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    ///
    /// # Errors
    /// Returns the connection layer error type if TCP fails.
    pub fn connect(host: &str, port: u16) -> Result<Self, crate::connection::ConnectionError> {
        Self::connect_with_timeout(host, port, DEFAULT_EXECUTE_TIMEOUT)
    }

    /// Connect to a Redis server with a custom default timeout.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    /// * `timeout` - Default timeout for all `execute()` calls
    ///
    /// # Errors
    /// Returns the connection layer error type if TCP fails.
    pub fn connect_with_timeout(
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<Self, crate::connection::ConnectionError> {
        let connection = Connection::connect(host, port)?;
        Ok(Self {
            inner: Arc::new(InnerClient {
                connection,
                default_timeout: timeout,
                command_policy: crate::protocol::builder::CommandPolicy::AllowAll,
            }),
        })
    }

    /// Connect to a Redis server with SSRF protection enabled.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port
    /// * `timeout` - Default timeout for all `execute()` calls
    /// * `ssrf_config` - Configuration for which IP ranges to block
    /// * `command_policy` - Policy for which Redis commands are allowed
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if DNS resolution, TCP connect, or SSRF
    /// check fails.
    pub fn connect_with_ssrf_protection(
        host: &str,
        port: u16,
        timeout: Duration,
        ssrf_config: SsrfConfig,
        command_policy: crate::protocol::builder::CommandPolicy,
    ) -> Result<Self, crate::connection::ConnectionError> {
        let connection =
            Connection::connect_with_ssrf_protection(host, port, timeout, ssrf_config)?;
        Ok(Self {
            inner: Arc::new(InnerClient {
                connection,
                default_timeout: timeout,
                command_policy,
            }),
        })
    }

    /// Returns the current command policy enforced by this client.
    #[must_use]
    pub fn command_policy(&self) -> &crate::protocol::builder::CommandPolicy {
        &self.inner.command_policy
    }

    /// Establish a TLS connection to a Redis server.
    ///
    /// # Arguments
    /// * `host` - Server hostname or IP address
    /// * `port` - Server port (typically 6380 for TLS)
    /// * `tls_config` - TLS configuration
    /// * `timeout_secs` - Connection timeout in seconds
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the TLS connection fails.
    #[cfg(feature = "tls")]
    pub fn connect_tls(
        host: &str,
        port: u16,
        tls_config: &crate::tls::TlsConfig,
        timeout_secs: u32,
    ) -> Result<Self, crate::connection::ConnectionError> {
        let connection = Connection::connect_tls(host, port, tls_config, timeout_secs)?;
        Ok(Self {
            inner: Arc::new(InnerClient {
                connection,
                default_timeout: Duration::from_secs(u64::from(timeout_secs)),
                command_policy: crate::protocol::builder::CommandPolicy::AllowAll,
            }),
        })
    }

    /// Connect to a Redis server given a URL.
    ///
    /// # Supported formats
    ///
    /// * `redis://:password@host:port` — plain TCP with AUTH (Redis < 6)
    /// * `redis://user:***@host:port` — plain TCP with username + password
    /// * `rediss://host:port` — TLS (port defaults to 6380)
    /// * `rediss://:password@host:port` — TLS + AUTH
    ///
    /// # TLS support (rediss://)
    ///
    /// TLS URLs use `--features tls` at build time. Query parameters:
    ///
    /// * `timeout=N` — connection timeout in seconds (default: 5)
    /// * `ca_cert=/path/to/ca.pem` — custom CA certificate path(s), comma-separated
    /// * `client_cert=/path/to/client.pem` — client certificate for mTLS
    /// * `client_key=/path/to/client-key.pem` — client private key for mTLS
    /// * `verify_server=true|false` — disable hostname verification (default: true)
    ///
    /// # URL encoding
    ///
    /// Passwords and usernames are URL-decoded before use. This allows
    /// passwords containing `@`, `:`, `/`, `?`, `#`, `[`, `]`, `%` to be
    /// represented in URLs via percent-encoding.
    ///
    /// # Errors
    ///
    /// Returns [`RedisError::Parse`] if the URL has an unsupported scheme,
    /// invalid port, unclosed IPv6 bracket, double prefix, or if the AUTH
    /// command fails after a successful connection.
    pub fn connect_url(url: &str) -> Result<Self, RedisError> {
        client_url::connect_url(url)
    }

    /// Execute a command and return the typed result.
    ///
    /// Uses the default timeout configured when the client was created
    /// (default: 5 seconds via [`Self::connect_with_timeout`]).
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
        self.execute_with_timeout(cmd, self.inner.default_timeout)
    }

    /// Send a PING command and return the response.
    ///
    /// # Errors
    /// Returns [`RedisError`] if the server does not respond with "PONG".
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

// Implement domain traits on RedisClient.
// Each impl is empty — the blanket impl in commands/mod.rs gives
// `Commands` to anything implementing all 8 domain traits, so
// method overrides come from the default impls in the domain trait files.
impl StringsCommands for RedisClient {}
impl HashesCommands for RedisClient {}
impl SetsCommands for RedisClient {}
impl ListsCommands for RedisClient {}
impl SortedSetsCommands for RedisClient {}
impl PubsubCommands for RedisClient {}
impl TransactionsCommands for RedisClient {}
impl AdminCommands for RedisClient {}

// Note: `Commands` is impl'd on `RedisClient` only.
// `&RedisClient` gets it automatically via auto-deref — no separate impl needed.
// The only exception is `ping`: the inherent `ping()` returns `Result<String, RedisError>`
// (executes the command), while `Commands::ping()` returns `CommandBuilder` (builds it).
// Auto-deref resolves `&RedisClient::ping()` to the *inherent* method, which is the
// expected behavior — callers wanting the raw builder use `Commands::ping()`.
