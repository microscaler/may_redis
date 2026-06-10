// Pipeline — Batch command execution for may-redis
//
// Provides the `Pipeline` struct for sending multiple Redis commands
// in a single batch.
//
// Pipeline mirrors the redis-rs pipeline pattern:
// 1. Build commands with `add()`
// 2. Execute all at once with `execute()`
// 3. Responses come back in order

use crate::connection::{Connection, ConnectionLimitError, Request};
use crate::core::{RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use may::coroutine::yield_now;

fn map_send_error(e: &ConnectionLimitError) -> RedisError {
    RedisError::Connection(format!("pipeline send failed: {e}"))
}

/// Batch command execution.
///
/// Commands are sent to the server and responses are collected.
/// The pipeline is automatically flushed when it goes out of scope.
///
/// # Example
///
/// ```no_run
/// use may_redis::{RedisClient, Pipeline};
///
/// let client = RedisClient::connect("127.0.0.1", 6379).unwrap();
/// let mut pipeline = client.pipeline();
/// // Add commands with `pipeline.add(builder)`; see Commands trait for builders.
/// ```
pub struct Pipeline<'a> {
    connection: &'a Connection,
    commands: Vec<Vec<u8>>,
    senders: Vec<may::sync::spsc::Sender<crate::core::RedisValue>>,
    receivers: Vec<may::sync::spsc::Receiver<crate::core::RedisValue>>,
}

impl<'a> Pipeline<'a> {
    /// Create an empty pipeline backed by the given connection.
    #[must_use]
    pub const fn new(connection: &'a Connection) -> Self {
        Self {
            connection,
            commands: Vec::new(),
            senders: Vec::new(),
            receivers: Vec::new(),
        }
    }

    /// Add a command to the pipeline.
    ///
    /// # Panics
    ///
    /// Panics if the command is blocked by the default [`CommandPolicy`].
    /// This is by design: blocked commands should be caught at build time.
    #[allow(clippy::unwrap_used)]
    pub fn add(&mut self, cmd: CommandBuilder) {
        // Encode the command into RESP bytes
        // AC-3.11: build() returns None if the command is blocked by the CommandPolicy.
        // We use unwrap() here because invalid commands should be caught at build time,
        // not silently dropped in a pipeline.
        let data = cmd.build().unwrap().to_vec();
        // Create a spsc channel for this command's response
        let (tx, rx) = may::sync::spsc::channel();
        // Queue the command and store sender/receiver pair
        self.commands.push(data);
        self.senders.push(tx);
        self.receivers.push(rx);
    }

    /// Execute all queued commands and collect raw `RedisValue` responses.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] if a command cannot be queued on
    /// the connection (queue full, request too large) or if the response
    /// channel closes before a response arrives.
    pub fn execute_raw(&mut self) -> Result<Vec<RedisValue>, RedisError> {
        for (data, tx) in std::mem::take(&mut self.commands)
            .into_iter()
            .zip(std::mem::take(&mut self.senders))
        {
            let request = Request::new(data, tx);
            self.connection
                .send(request)
                .map_err(|e| map_send_error(&e))?;
        }

        yield_now();

        let mut responses = Vec::with_capacity(self.receivers.len());
        for rx in std::mem::take(&mut self.receivers) {
            yield_now();
            let response = rx.recv().map_err(|_| {
                RedisError::Connection("response channel closed".into())
            })?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// Execute all queued commands and collect responses as individual results.
    ///
    /// Unlike `execute_raw()`, this returns `Vec<Result<RedisValue, RedisError>>`
    /// so that individual command failures don't fail the entire pipeline.
    /// A send rejected by connection limits or a closed response channel
    /// produces an `Err` entry for that command instead of waiting forever.
    ///
    /// Responses are dispatched in FIFO order by the connection loop, so
    /// receivers are drained in order with blocking `recv()` — which parks
    /// the coroutine. Do NOT replace this with a `try_recv()` polling loop:
    /// a yield-spinning coroutine hogs its may worker and starves the
    /// connection-loop coroutine (Bug 1 in
    /// `llmwiki/topics/connection-loop-pitfalls.md`).
    pub fn execute_raw_results(&mut self) -> Vec<Result<RedisValue, RedisError>> {
        let n = self.commands.len();

        // Push all commands to the connection's request queue at once.
        // A failed send is recorded immediately: its response can never
        // arrive, so waiting on it would block forever.
        let mut send_errors: Vec<Option<RedisError>> = Vec::with_capacity(n);
        for (data, tx) in std::mem::take(&mut self.commands)
            .into_iter()
            .zip(std::mem::take(&mut self.senders))
        {
            let request = Request::new(data, tx);
            send_errors.push(
                self.connection
                    .send(request)
                    .err()
                    .map(|e| map_send_error(&e)),
            );
        }

        let receivers = std::mem::take(&mut self.receivers);

        // Yield to let the connection loop process all queued requests
        yield_now();

        receivers
            .into_iter()
            .zip(send_errors)
            .map(|(rx, send_error)| {
                if let Some(e) = send_error {
                    return Err(e);
                }
                rx.recv().map_err(|_| {
                    RedisError::Connection("response channel closed".into())
                })
            })
            .collect()
    }

    /// Execute all queued commands and decode typed results via `FromPipelineResponse`.
    ///
    /// Use [`Self::execute_raw`] or [`Self::execute_raw_results`] to receive
    /// server errors inline as [`RedisValue::Error`] values instead.
    ///
    /// # Errors
    /// Returns [`RedisError::Protocol`] if any response in the batch is a
    /// server error (`-ERR ...`), carrying the server's message.
    /// Returns [`crate::core::RedisError::Parse`] if the number of responses does not match
    /// the expected count for the target type, or if a response cannot be
    /// converted to the requested Rust type. Delegates to the underlying
    /// `execute_raw()` which can also return connection errors.
    pub fn execute<T: super::pipeline_response::FromPipelineResponse>(
        &mut self,
    ) -> Result<T, crate::core::RedisError> {
        let responses = self.execute_raw()?;
        // Surface server errors loudly: depending on the target type a
        // RedisValue::Error could otherwise be swallowed or surface as a
        // confusing type-conversion failure.
        for value in &responses {
            if let RedisValue::Error(msg) = value {
                return Err(RedisError::Protocol(msg.clone()));
            }
        }
        T::from_responses(responses)
    }
}
