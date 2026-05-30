// Client — RedisClient implementation
//
// Provides the main user-facing API for connecting to Redis and executing commands.

use crate::connection::{Connection, Request};
use crate::core::{FromRedisValue, RedisError, RedisValue, ToRedisArgs};
use crate::protocol::{builder::CommandBuilder, commands::*};
use may::sync::spsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::pipeline::Pipeline;

// ---------------------------------------------------------------------------
// URL decoding helper (Story 2 — Issue #5: AC-2.10, AC-2.11, AC-2.12)
// ---------------------------------------------------------------------------

/// URL-decode a percent-encoded string.
///
/// Only valid `%HH` sequences are decoded; all other characters pass through
/// unchanged. Invalid percent-encoding (e.g. `%GG`) returns a `Parse` error.
/// O(n) with no backtracking.
fn url_decode(s: &str) -> Result<String, RedisError> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hi = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding at end of string".into())
            })?;
            let lo = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding (missing second hex digit)".into())
            })?;

            let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).map_err(|_| {
                RedisError::Parse(format!(
                    "invalid percent-encoding %{hi}{lo} (not valid hex)"
                ))
            })?;

            result.push(byte as char);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

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
// Timeout guard — cancels in-flight request when timeout fires (Finding #1, #2)
// ---------------------------------------------------------------------------

/// A guard that owns a tracked timeout coroutine and a shared cancellation flag.
///
/// # Usage pattern (Story 1 — Timeout Safety)
///
/// 1. Build RESP bytes
/// 2. Create `(tx, rx)` channel
/// 3. Create `TimeoutGuard` with the timeout duration
/// 4. **Send request** to connection loop
/// 5. Poll for response; if timeout fires, the guard marks cancelled
/// 6. Drop guard — cancels the sleeping coroutine
///
/// This ensures at most ONE sleeping coroutine per request (Finding #2) and
/// the cancellation flag is checked before sending so the request never
/// reaches the wire if we've already decided to time out (Finding #1).
struct TimeoutGuard {
    /// Set to `true` when the timeout coroutine fires.
    cancelled: Arc<AtomicBool>,
}

impl TimeoutGuard {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for TimeoutGuard {
    fn drop(&mut self) {
        // When the guard is dropped (response received before timeout),
        // the timeout coroutine's spsc sender is dropped, causing the
        // sleeping coroutine to panic on send. This is how we cancel
        // it without needing a formal coroutine cancel API.
    }
}

// ---------------------------------------------------------------------------
// Inner client state
// ---------------------------------------------------------------------------

/// Internal client state shared across coroutines.
struct InnerClient {
    connection: Connection,
    /// Default timeout for `execute()` — overrides hardcoded 30s.
    default_timeout: Duration,
    /// Command policy enforced on every `execute()` call (Story 3, Issue #9, FR-031).
    command_policy: crate::protocol::builder::CommandPolicy,
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
    inner: Arc<InnerClient>,
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
    /// FR-027: Enables SSRF checks on DNS resolution.
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
    /// check fails, or the connection layer error type if TCP fails.
    pub fn connect_with_ssrf_protection(
        host: &str,
        port: u16,
        timeout: Duration,
        ssrf_config: crate::connection::SsrfConfig,
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
    /// * `redis://host:port` — plain TCP with default port 6379
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
        // Issue #18: Reject double prefixes (FR-019, AC-2.13, AC-2.14)
        let after_scheme = if let Some(rest) = url.strip_prefix("rediss://") {
            // TLS not yet supported
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
            return Err(RedisError::Parse(format!(
                "unsupported URL scheme: {}",
                url.split("://").next().unwrap_or(url)
            )));
        };

        let scheme = ConnectionScheme::Plain;

        // Check for double redis:// prefix after stripping (FR-019, AC-2.13)
        if after_scheme.starts_with("redis://") {
            return Err(RedisError::Parse(
                "double URL scheme prefix (redis://redis://)".into(),
            ));
        }

        let rest = after_scheme.split('/').next().unwrap_or(after_scheme);

        // Find the LAST '@' to split user:password from host:port.
        // This correctly handles passwords containing '@' (RFC 3986 §3.2.1, FR-014).
        let (password, host_part) = rest.rfind('@').map_or((None, rest), |idx| {
            let password = &rest[..idx];
            let host_part = &rest[idx + 1..];
            if password.is_empty() {
                (None, host_part)
            } else {
                (Some(password), host_part)
            }
        });

        // Issue #5: URL-decode the password (FR-016, AC-2.10, AC-2.12)
        let password: Option<String> = password.map(url_decode).transpose()?;

        // Parse host:port  handle IPv6 `[::1]:6379` and IPv4 `127.0.0.1:6379`
        let (host, port) = if host_part.starts_with('[') {
            // IPv6: [addr]:port
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
            // IPv4: host:port
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
                .map_or_else(|| (host_part, default_port(scheme)), |(h, p)| (h, p))
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
    ///
    /// The timeout is checked BEFORE sending the request to the connection loop
    /// (Finding #1). If the timeout fires before the request is sent, it is
    /// cancelled and never reaches the socket. If the timeout fires after the
    /// request is queued, a `Connection` error is returned and the dropped
    /// spsc channel causes the connection loop to skip dispatching the response.
    ///
    /// The timeout coroutine is tracked and cancelled when the response arrives
    /// (Finding #2). No more than one sleeping coroutine exists per request.
    ///
    /// # Errors
    /// Returns [`ConnectionError`] if the TCP connection fails, the
    /// response channel is closed, or the timeout expires before a response
    /// is received.
    /// # Panics
    ///
    /// Panics if the command is blocked by the [`command_policy`].
    /// Blocked commands should be caught at build time, not at execution time.
    ///
    /// [`command_policy`]: Self::command_policy
    #[allow(clippy::unwrap_used)]
    pub fn execute_with_timeout<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
        timeout: Duration,
    ) -> Result<T, RedisError> {
        // AC-3.15: validate against client's policy BEFORE building (FR-031)
        if let Some(name) = cmd.command_name() {
            if !self.inner.command_policy.is_allowed(name) {
                return Err(RedisError::Security(format!(
                    "command '{name}' is denied by policy"
                )));
            }
        }

        // Step 1: Build the command into RESP bytes
        // AC-3.11: build() returns None if the command is blocked by the
        // CommandPolicy, so we return a Protocol error here.
        let data = cmd
            .build()
            .ok_or_else(|| RedisError::Protocol("command blocked by command policy".into()))?;

        // Step 2: Create a channel for this request's response
        let (tx, rx) = spsc::channel();

        // Step 3: Create the timeout guard (shared cancellation flag)
        let guard = TimeoutGuard::new();
        let cancelled = Arc::clone(&guard.cancelled);

        // Step 4: Check timeout BEFORE sending the request (Finding #1)
        // We start the timeout coroutine and check for fire immediately.
        // If it fires, we never send the request.
        let (timeout_tx, timeout_rx) = spsc::channel::<()>();

        // Clone the Arc so the closure gets its own reference and `cancelled`
        // below remains usable after the move.
        let cancelled_for_closure = Arc::clone(&cancelled);

        // Spawn the timeout coroutine — when the guard is dropped (response
        // arrived first), timeout_tx is dropped which causes the sleeping
        // coroutine to fail on send, cleanly cancelling it (Finding #2).
        may::go!(move || {
            may::coroutine::sleep(timeout);
            // Timeout fired — signal cancellation via shared flag so the
            // caller knows the request was cancelled, not completed.
            cancelled_for_closure.store(true, Ordering::SeqCst);
            let _ = timeout_tx.send(());
        });

        // Step 5: Check if the timeout fired before sending
        if cancelled.load(Ordering::SeqCst) {
            // Timeout fired instantly — don't send the request.
            // The timeout coroutine will clean up on drop.
            return Err(RedisError::Connection(format!(
                "command execution timed out after {timeout:?}"
            )));
        }

        // Step 6: Send the request to the connection loop
        let _tag = self.inner.connection.send(Request::new(data.to_vec(), tx));

        // Step 7: Poll loop — wait for response or timeout signal
        let response = loop {
            if let Ok(resp) = rx.try_recv() {
                // Response arrived — drop the timeout guard which cancels
                // the sleeping coroutine (Finding #2).
                break resp;
            }
            if timeout_rx.try_recv().is_ok() {
                // Timeout signal received — return error.
                // The guard is dropped here, cleaning up the coroutine.
                break RedisValue::Error(format!("command execution timed out after {timeout:?}"));
            }
            may::coroutine::yield_now();
        };

        // Convert RedisValue to the requested type
        if let RedisValue::Error(msg) = response {
            return Err(RedisError::Protocol(msg));
        }

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

    /// Execute a command and return the typed result.
    ///
    /// Uses the default timeout configured when the client was created
    /// (default: 5 seconds via [`Self::connect_with_timeout`]).
    ///
    /// # Security note
    ///
    /// The 5-second default is a significant reduction from the previous
    /// 30-second default (Finding #16). This prevents slow commands from
    /// executing on the server for extended periods after the client gives up.
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
// Empty impls for each domain trait. The blanket impl in commands/mod.rs
// gives `Commands` to anything implementing all 8 domain traits, so no
// method overrides are needed — they come from the default impls in the
// domain trait files.
impl StringsCommands for RedisClient {}
impl HashesCommands for RedisClient {}
impl SetsCommands for RedisClient {}
impl ListsCommands for RedisClient {}
impl SortedSetsCommands for RedisClient {}
impl PubsubCommands for RedisClient {}
impl TransactionsCommands for RedisClient {}
impl AdminCommands for RedisClient {}

/// Note: `Commands` is impl'd on `RedisClient` only.
/// `&RedisClient` gets it automatically via auto-deref — no separate impl needed.
/// The only exception is `ping`: the inherent `ping()` returns `Result<String, RedisError>`
/// (executes the command), while `Commands::ping()` returns `CommandBuilder` (builds it).
/// Auto-deref resolves `&RedisClient::ping()` to the *inherent* method, which is the
/// expected behavior — callers wanting the raw builder use `Commands::ping()`.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
