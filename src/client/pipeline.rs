// Pipeline — Batch command execution for may-redis
//
// Provides the `Pipeline` struct for sending multiple Redis commands
// in a single batch, and the `FromPipelineResponse` trait for extracting
// typed results from the collected responses.
//
// Pipeline mirrors the redis-rs pipeline pattern:
// 1. Build commands with `add()`
// 2. Execute all at once with `execute()`
// 3. Responses come back in order

use crate::connection::{Connection, Request};
use crate::core::{FromRedisValue, RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use may::coroutine::yield_now;
use may::sync::spsc;
// Arc and Mutex are used by the InMemoryClient test double, not here

/// Trait for extracting typed results from multiple pipeline responses.
///
/// Implemented for single tuples `(T1,)`, pairs `(T1, T2)`, triples
/// `(T1, T2, T3)`, and `Vec<T>` to cover the most common pipeline use cases.
pub trait FromPipelineResponse: Sized {
    /// Convert a vector of `RedisValue` responses into `Self`.
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError>;
}

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
    senders: Vec<spsc::Sender<RedisValue>>,
    receivers: Vec<spsc::Receiver<RedisValue>>,
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
    /// The command is encoded into RESP bytes and queued for batch
    /// execution. Responses will arrive in the same order as commands
    /// were added.
    pub fn add(&mut self, cmd: CommandBuilder) {
        // Encode the command into RESP bytes
        let data = cmd.build().to_vec();
        // Create a spsc channel for this command's response
        let (tx, rx) = spsc::channel();
        // Queue the command and store sender/receiver pair
        self.commands.push(data);
        self.senders.push(tx);
        self.receivers.push(rx);
    }

    /// Execute all queued commands and collect raw `RedisValue` responses.
    pub fn execute_raw(&mut self) -> Result<Vec<RedisValue>, RedisError> {
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
                .map_err(|_| RedisError::Parse("response channel closed".into()))?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// Execute all queued commands and collect responses as individual results.
    ///
    /// Unlike `execute_raw()`, this returns `Vec<Result<RedisValue, RedisError>>`
    /// so that individual command failures don't block the entire pipeline.
    pub fn execute_raw_results(&mut self) -> Vec<Result<RedisValue, RedisError>> {
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
    pub fn execute<T: FromPipelineResponse>(&mut self) -> Result<T, RedisError> {
        let responses = self.execute_raw()?;
        T::from_responses(responses)
    }
}

// ---------------------------------------------------------------------------
// FromPipelineResponse implementations
// ---------------------------------------------------------------------------

impl<T1: FromRedisValue> FromPipelineResponse for (T1,) {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 1 {
            return Err(RedisError::Parse(format!(
                "expected 1 response, got {}",
                responses.len()
            )));
        }
        let t1 = T1::from_redis_value(&responses.into_iter().next().unwrap())?;
        Ok((t1,))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue> FromPipelineResponse for (T1, T2) {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 2 {
            return Err(RedisError::Parse(format!(
                "expected 2 responses, got {}",
                responses.len()
            )));
        }
        let mut iter = responses.into_iter();
        let t1 = T1::from_redis_value(&iter.next().unwrap())?;
        let t2 = T2::from_redis_value(&iter.next().unwrap())?;
        Ok((t1, t2))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue, T3: FromRedisValue> FromPipelineResponse
    for (T1, T2, T3)
{
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 3 {
            return Err(RedisError::Parse(format!(
                "expected 3 responses, got {}",
                responses.len()
            )));
        }
        let mut iter = responses.into_iter();
        let t1 = T1::from_redis_value(&iter.next().unwrap())?;
        let t2 = T2::from_redis_value(&iter.next().unwrap())?;
        let t3 = T3::from_redis_value(&iter.next().unwrap())?;
        Ok((t1, t2, t3))
    }
}

impl<T1: FromRedisValue, T2: FromRedisValue, T3: FromRedisValue, T4: FromRedisValue>
    FromPipelineResponse for (T1, T2, T3, T4)
{
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        if responses.len() != 4 {
            return Err(RedisError::Parse(format!(
                "expected 4 responses, got {}",
                responses.len()
            )));
        }
        let mut iter = responses.into_iter();
        let t1 = T1::from_redis_value(&iter.next().unwrap())?;
        let t2 = T2::from_redis_value(&iter.next().unwrap())?;
        let t3 = T3::from_redis_value(&iter.next().unwrap())?;
        let t4 = T4::from_redis_value(&iter.next().unwrap())?;
        Ok((t1, t2, t3, t4))
    }
}

impl<T: FromRedisValue> FromPipelineResponse for Vec<T> {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError> {
        let mut result = Self::with_capacity(responses.len());
        for response in responses {
            result.push(T::from_redis_value(&response)?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_pipeline_response_single() {
        let responses = vec![RedisValue::Integer(42)];
        let result: Result<(i64,), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (42,));
    }

    #[test]
    fn test_from_pipeline_response_pair() {
        let responses = vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(b"hello".to_vec()),
        ];
        let result: Result<(bool, String), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (true, "hello".to_string()));
    }

    #[test]
    fn test_from_pipeline_response_triple() {
        let responses = vec![
            RedisValue::Integer(1),
            RedisValue::Integer(2),
            RedisValue::Integer(3),
        ];
        let result: Result<(bool, i64, i64), _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(result.unwrap(), (true, 2, 3));
    }

    #[test]
    fn test_from_pipeline_response_vec() {
        let responses = vec![
            RedisValue::BulkString(b"a".to_vec()),
            RedisValue::BulkString(b"b".to_vec()),
            RedisValue::BulkString(b"c".to_vec()),
        ];
        let result: Result<Vec<String>, _> = FromPipelineResponse::from_responses(responses);
        assert_eq!(
            result.unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_from_pipeline_response_wrong_count() {
        let responses = vec![RedisValue::Integer(1)];
        let result: Result<(i64, i64), _> = FromPipelineResponse::from_responses(responses);
        assert!(result.is_err());
    }
}
