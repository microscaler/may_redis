//! Dedicated pub/sub client — separate connection for SUBSCRIBE/PSUBSCRIBE.
//!
//! Normal [`RedisClient`] must not be used for subscriptions; use
//! [`PubSubClient`] instead so push messages are routed correctly.

use std::sync::Arc;
use std::time::Duration;

use crate::connection::{Connection, PubSubMessage, Request};
use crate::core::{RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use crate::protocol::commands::PubsubCommands;
use may::sync::spsc;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

struct InnerPubSubClient {
    connection: Connection,
    message_rx: spsc::Receiver<PubSubMessage>,
    default_timeout: Duration,
}

/// Subscriber connection for Redis pub/sub.
///
/// Owns a dedicated TCP connection. After `subscribe` / `psubscribe`,
/// call [`Self::recv_message`] to receive published payloads.
#[derive(Clone)]
pub struct PubSubClient {
    inner: Arc<InnerPubSubClient>,
}

impl PubSubClient {
    /// Connect to Redis for pub/sub on the given host and port.
    ///
    /// # Errors
    /// Returns [`ConnectionError`](crate::connection::ConnectionError) on TCP failure.
    pub fn connect(
        host: &str,
        port: u16,
    ) -> Result<Self, crate::connection::ConnectionError> {
        let (push_tx, push_rx) = spsc::channel();
        let connection = Connection::connect_for_pubsub(host, port, push_tx)?;
        Ok(Self {
            inner: Arc::new(InnerPubSubClient {
                connection,
                message_rx: push_rx,
                default_timeout: DEFAULT_TIMEOUT,
            }),
        })
    }

    /// Subscribe to channels (one Redis command per channel).
    ///
    /// # Errors
    /// Returns [`RedisError`] if a command fails or times out.
    pub fn subscribe<K: crate::core::ToRedisArgs>(
        &self,
        channels: &[K],
    ) -> Result<(), RedisError> {
        for ch in channels {
            let cmd = PubsubCommands::subscribe(self, std::slice::from_ref(ch));
            self.execute_value(cmd)?;
        }
        Ok(())
    }

    /// Pattern-subscribe (one Redis command per pattern).
    ///
    /// # Errors
    /// Returns [`RedisError`] if a command fails or times out.
    pub fn psubscribe<K: crate::core::ToRedisArgs>(
        &self,
        patterns: &[K],
    ) -> Result<(), RedisError> {
        for pattern in patterns {
            let cmd = PubsubCommands::psubscribe(self, std::slice::from_ref(pattern));
            self.execute_value(cmd)?;
        }
        Ok(())
    }

    /// Unsubscribe from all channels.
    ///
    /// # Errors
    /// Returns [`RedisError`] if the command fails or times out.
    pub fn unsubscribe_all(&self) -> Result<(), RedisError> {
        self.execute_value(PubsubCommands::unsubscribe(self))?;
        Ok(())
    }

    /// Unsubscribe from specific channels (one command per channel).
    ///
    /// # Errors
    /// Returns [`RedisError`] if a command fails or times out.
    pub fn unsubscribe_channels<K: crate::core::ToRedisArgs>(
        &self,
        channels: &[K],
    ) -> Result<(), RedisError> {
        for ch in channels {
            let cmd =
                PubsubCommands::unsubscribe_channels(self, std::slice::from_ref(ch));
            self.execute_value(cmd)?;
        }
        Ok(())
    }

    /// Block until the next pub/sub push message arrives.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] on timeout or channel close.
    pub fn recv_message(&self) -> Result<PubSubMessage, RedisError> {
        self.recv_message_timeout(self.inner.default_timeout)
    }

    /// Block until the next pub/sub push message or timeout.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] on timeout or channel close.
    pub fn recv_message_timeout(
        &self,
        timeout: Duration,
    ) -> Result<PubSubMessage, RedisError> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Ok(msg) = self.inner.message_rx.try_recv() {
                return Ok(msg);
            }
            if std::time::Instant::now() >= deadline {
                return Err(RedisError::Connection(format!(
                    "pub/sub recv timed out after {timeout:?}"
                )));
            }
            may::coroutine::yield_now();
        }
    }

    fn execute_value(&self, cmd: CommandBuilder) -> Result<RedisValue, RedisError> {
        let data = cmd
            .build()
            .ok_or_else(|| RedisError::Protocol("command blocked by policy".into()))?;
        let (tx, rx) = spsc::channel();
        self.inner
            .connection
            .send(Request::new(data.to_vec(), tx))
            .map_err(|e| RedisError::Connection(format!("send failed: {e}")))?;
        let deadline = std::time::Instant::now() + self.inner.default_timeout;
        loop {
            if let Ok(value) = rx.try_recv() {
                if let RedisValue::Error(msg) = value {
                    return Err(RedisError::Protocol(msg));
                }
                return Ok(value);
            }
            if std::time::Instant::now() >= deadline {
                return Err(RedisError::Connection(format!(
                    "command timed out after {:?}",
                    self.inner.default_timeout
                )));
            }
            may::coroutine::yield_now();
        }
    }
}

impl PubsubCommands for PubSubClient {}
