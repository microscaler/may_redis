// Pipeline — Batch command execution for may-redis
//
// Provides the `Pipeline` struct for sending multiple Redis commands
// in a single batch.
//
// Pipeline mirrors the redis-rs pipeline pattern:
// 1. Build commands with `add()`
// 2. Execute all at once with `execute()`
// 3. Responses come back in order

use crate::connection::{Connection, Request};
use crate::protocol::builder::CommandBuilder;
use may::coroutine::yield_now;

/// Batch command execution.
///
/// `Pipeline` borrows a `RedisClient` and accumulates encoded RESP commands.
/// Calling `execute()` sends all accumulated commands at once and collects
/// responses in order.
///
/// # Example
///
/// ```no_run
/// use may_redis::{RedisClient, Pipeline, Commands};
///
/// let client = RedisClient::connect("127.0.0.1", 6379).unwrap();
/// let mut pipeline = client.pipeline();
/// pipeline.add(client.set("key1", "value1"));
/// pipeline.add(client.set("key2", "value2"));
/// let results: ((), ()) = pipeline.execute().unwrap();
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
    /// Returns [`crate::core::RedisError::Parse`] if the response channel is closed.
    pub fn execute_raw(&mut self) -> Result<Vec<crate::core::RedisValue>, crate::core::RedisError> {
        // Push all commands to the connection's request queue at once
        // using the senders we stored during `add()`
        for (data, tx) in std::mem::take(&mut self.commands)
            .into_iter()
            .zip(std::mem::take(&mut self.senders))
        {
            let request = Request::new(data, tx);
            let _ = self.connection.send(request);
        }

        // Yield to let the connection loop process all queued requests
        // before we start collecting responses. Without this, the first
        // rx.recv() would block the coroutine before the epoll loop
        // has a chance to process the queued commands and send responses.
        yield_now();

        // Collect responses from the receivers we stored during `add()`
        let mut responses = Vec::with_capacity(self.receivers.len());
        for rx in std::mem::take(&mut self.receivers) {
            let response = rx
                .recv()
                .map_err(|_| crate::core::RedisError::Parse("response channel closed".into()))?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// Execute all queued commands and collect responses as individual results.
    ///
    /// Unlike `execute_raw()`, this returns `Vec<Result<RedisValue, RedisError>>`
    /// so that individual command failures don't block the entire pipeline.
    pub fn execute_raw_results(
        &mut self,
    ) -> Vec<Result<crate::core::RedisValue, crate::core::RedisError>> {
        let n = self.commands.len();

        // Push all commands to the connection's request queue at once
        for (data, tx) in std::mem::take(&mut self.commands)
            .into_iter()
            .zip(std::mem::take(&mut self.senders))
        {
            let request = Request::new(data, tx);
            let _ = self.connection.send(request);
        }

        // Drain receivers into a local vec so we can poll them
        let receivers = std::mem::take(&mut self.receivers);

        // Yield to let the connection loop process all queued requests
        yield_now();

        // Poll each receiver with try_recv, yielding between rounds.
        // This avoids blocking on any single command.
        let mut results = vec![None; n];
        let mut done = 0;
        while done < n {
            for i in 0..n {
                if results[i].is_none() {
                    if let Ok(val) = receivers[i].try_recv() {
                        results[i] = Some(Ok(val));
                        done += 1;
                    }
                }
            }
            if done < n {
                yield_now();
            }
        }

        results.into_iter().flatten().collect()
    }

    /// Execute all queued commands and decode typed results via `FromPipelineResponse`.
    ///
    /// # Errors
    /// Returns [`crate::core::RedisError::Parse`] if the number of responses does not match
    /// the expected count for the target type, or if a response cannot be
    /// converted to the requested Rust type. Delegates to the underlying
    /// `execute_raw()` which can also return connection errors.
    pub fn execute<T: super::pipeline_response::FromPipelineResponse>(
        &mut self,
    ) -> Result<T, crate::core::RedisError> {
        let responses = self.execute_raw()?;
        T::from_responses(responses)
    }
}
