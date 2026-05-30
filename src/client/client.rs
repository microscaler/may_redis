// RedisClient — main entry point for Redis operations.
//
// Provides RedisClient struct with connect(), connect_with_timeout(),
// connect_with_ssrf_protection(), and connect_url() methods.

use std::sync::Arc;
use std::time::Duration;

use super::client_timeout::TimeoutGuard;
use super::client_url::url_decode;
use super::pipeline::Pipeline;
use crate::connection::{Connection, Request, SsrfConfig};
use crate::core::{FromRedisValue, RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use crate::protocol::commands::*;

// ---------------------------------------------------------------------------
// Connection scheme & helpers
// ---------------------------------------------------------------------------

/// Connection scheme: plain TCP or TLS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // TLS support planned for future epics
enum ConnectionScheme {
    Plain,
    Tls,
}

/// Return the default port for the given connection scheme.
const fn default_port(scheme: ConnectionScheme) -> u16 {
    match scheme {
        ConnectionScheme::Plain => 6379,
        ConnectionScheme::Tls => 6380,
    }
}

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

    /// Connect to a Redis server given a URL.
    ///
    /// # Supported formats
    ///
    /// * `redis://:password@host:port` — plain TCP with AUTH (Redis < 6)
    /// * `redis://user:password@host:port` — plain TCP with username + password
    /// * `rediss://host:port` — TLS (port defaults to 6380)
    /// * `rediss://:password@host:port` — TLS + AUTH
    ///
    /// # TLS support
    ///
    /// Currently `rediss://` URLs are rejected with a `Parse` error because
    /// TLS is not yet implemented.
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
        // Issue #18: Reject double prefixes
        let after_scheme = if let Some(rest) = url.strip_prefix("rediss://") {
            if rest.starts_with("rediss://") {
                return Err(RedisError::Parse(
                    "double URL scheme prefix (rediss://rediss://)".into(),
                ));
            }
            return Err(RedisError::Parse(
                "TLS is not yet supported (rediss://)".into(),
            ));
        } else if let Some(rest) = url.strip_prefix("redis://") {
            rest
        } else {
            return Err(RedisError::Parse(
                "must use 'redis://' prefix (no scheme given)".into(),
            ));
        };

        // Parse user:password@host:port — use rfind('@') to correctly handle
        // passwords containing '@' (RFC 3986 §3.2.1).
        let (password, host_part) = after_scheme.rfind('@').map_or((None, after_scheme), |idx| {
            let password = &after_scheme[..idx];
            let host_part = &after_scheme[idx + 1..];
            if password.is_empty() {
                (None, host_part)
            } else {
                (Some(password), host_part)
            }
        });

        // URL-decode the password
        let password: Option<String> = password.map(url_decode).transpose()?;

        // Parse host:port — handle IPv6 [::1]:6379 and IPv4 127.0.0.1:6379
        let (host, port) = if host_part.starts_with('[') {
            if let Some(close_bracket) = host_part.find(']') {
                let host = &host_part[1..close_bracket];
                let port_part = &host_part[close_bracket + 1..];
                let port: u16 = port_part
                    .strip_prefix(':')
                    .ok_or_else(|| RedisError::Parse("missing port for IPv6 address".into()))?
                    .parse()
                    .map_err(|e| RedisError::Parse(format!("invalid port: {e}")))?;
                (host, port)
            } else {
                return Err(RedisError::Parse("unclosed '[' in IPv6 address".into()));
            }
        } else {
            host_part
                .rfind(':')
                .map(|colon_idx| {
                    let host = &host_part[..colon_idx];
                    let port_str = &host_part[colon_idx + 1..];
                    let port: u16 = port_str
                        .parse()
                        .map_err(|e| RedisError::Parse(format!("invalid port: {e}")))?;
                    Ok::<_, RedisError>((host, port))
                })
                .transpose()?
                .map_or_else(
                    || (host_part, default_port(ConnectionScheme::Plain)),
                    |(h, p)| (h, p),
                )
        };

        // Use configurable default timeout
        let client = Self::connect(host, port)
            .map_err(|e| RedisError::Parse(format!("connection failed: {e}")))?;

        // Send AUTH if password was provided in URL
        if let Some(pass) = password {
            let auth_cmd = CommandBuilder::new("AUTH").arg(pass);
            client
                .execute::<String>(auth_cmd)
                .map_err(|e| RedisError::Parse(format!("AUTH failed: {e}")))?;
        }

        Ok(client)
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
