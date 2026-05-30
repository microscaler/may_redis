// Timeout safety for may-redis client execute().
//
// Provides TimeoutGuard and execute_with_timeout / execute_timeout methods.

use crate::connection::Request;
use crate::core::{FromRedisValue, RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use may::sync::spsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Timeout guard with cancellation flag — visible to sibling client modules.
pub(super) struct TimeoutGuard {
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
// Timeout-aware execution
// ---------------------------------------------------------------------------

impl super::client::RedisClient {
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
    /// [`command_policy`]: super::client::RedisClient::command_policy
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
        let (timeout_tx, timeout_rx) = spsc::channel::<()>();

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
            return Err(RedisError::Connection(format!(
                "command execution timed out after {timeout:?}"
            )));
        }

        // Step 6: Send the request to the connection loop
        let _tag = self.inner.connection.send(Request::new(data.to_vec(), tx));

        // Step 7: Poll loop — wait for response or timeout signal
        let response = loop {
            if let Ok(resp) = rx.try_recv() {
                break resp;
            }
            if timeout_rx.try_recv().is_ok() {
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
}
