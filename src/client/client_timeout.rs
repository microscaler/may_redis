// Timeout safety for may-redis client execute().
//
// Provides execute_with_timeout / execute_timeout methods.

use crate::connection::Request;
use crate::core::{FromRedisValue, RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use may::sync::spsc::{self, RecvError};
use std::time::Duration;

fn map_recv_error(e: &RecvError) -> RedisError {
    match e {
        RecvError::Timeout => RedisError::Connection("timeout".into()),
        RecvError::Disconnected => {
            RedisError::Connection("response channel closed".into())
        }
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
    /// * `timeout` - Maximum time to wait for the response on the spsc channel
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] with message `"timeout"` when the
    /// deadline elapses before a response arrives, or when the response channel
    /// closes.
    #[allow(clippy::unwrap_used)]
    pub fn execute_with_timeout<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
        timeout: Duration,
    ) -> Result<T, RedisError> {
        if let Some(name) = cmd.command_name() {
            if !self.inner.command_policy.is_allowed(name) {
                return Err(RedisError::Security(format!(
                    "command '{name}' is denied by policy"
                )));
            }
        }

        let data = cmd.build().ok_or_else(|| {
            RedisError::Protocol("command blocked by command policy".into())
        })?;

        let (tx, rx) = spsc::channel();
        let _tag = self.inner.connection.send(Request::new(data.to_vec(), tx));

        // Yield so the connection loop can flush before we block on recv
        // (same pattern as `Pipeline::execute_raw` and may_postgres).
        may::coroutine::yield_now();

        let response = rx
            .recv_with_timeout(timeout)
            .map_err(|e| map_recv_error(&e))?;

        if let RedisValue::Error(msg) = response {
            return Err(RedisError::Protocol(msg));
        }

        T::from_redis_value(&response)
    }

    /// Execute a command with a timeout in seconds and return the typed result.
    ///
    /// # Errors
    ///
    /// Returns [`RedisError`] if the command fails, the response cannot be
    /// decoded, the deadline elapses, or the connection returns an error value.
    pub fn execute_timeout<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
        seconds: u32,
    ) -> Result<T, RedisError> {
        self.execute_with_timeout(cmd, Duration::from_secs(u64::from(seconds)))
    }
}
