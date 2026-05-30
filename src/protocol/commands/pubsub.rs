// Commands — pubsub trait
//
// Provides all Pubsub commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Pubsub command methods.
pub trait PubsubCommands: Sized {
    /// PUBLISH channel message — Publish a message to a channel.
    ///
    /// # Warning: pub/sub requires a dedicated connection
    ///
    /// `SUBSCRIBE`, `PSUBSCRIBE`, `UNSUBSCRIBE`, and `PUNSUBSCRIBE` put the
    /// connection into a *subscription state* where the Redis server sends
    /// unsolicited messages (the published payloads) to the client. This
    /// means:
    /// - The connection loop **must** handle both request-response messages
    ///   and incoming pub/sub messages on the same socket.
    /// - **This client does not yet support pub/sub.** A subscribe call will
    ///   put the connection in a state where normal request-response
    ///   correlation breaks, because messages arrive out of order.
    /// - **Do NOT use** `subscribe`, `psubscribe`, `unsubscribe`, or
    ///   `punsubscribe` with this client. They will likely cause data loss
    ///   or deadlocks.
    ///
    /// `PUBLISH` (fire-and-forget) is safe because it does not change the
    /// connection state — it just sends a command and returns the number
    /// of subscribers.
    #[must_use = "call .build() to encode the command"]
    fn publish<K: ToRedisArgs, M: ToRedisArgs>(&self, channel: K, message: M) -> CommandBuilder {
        CommandBuilder::new("PUBLISH").arg(channel).arg(message)
    }

    /// SUBSCRIBE channel [channel ...] — Subscribe to channels
    #[must_use = "call .build() to encode the command"]
    fn subscribe<K: ToRedisArgs>(&self, channels: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("SUBSCRIBE");
        for ch in channels {
            builder = builder.arg(ch);
        }
        builder
    }

    /// UNSUBSCRIBE — Unsubscribe from all channels
    #[must_use = "call .build() to encode the command"]
    fn unsubscribe(&self) -> CommandBuilder {
        CommandBuilder::new("UNSUBSCRIBE")
    }

    /// UNSUBSCRIBE channel [channel ...] — Unsubscribe from specific channels
    #[must_use = "call .build() to encode the command"]
    fn unsubscribe_channels<K: ToRedisArgs>(&self, channels: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("UNSUBSCRIBE");
        for ch in channels {
            builder = builder.arg(ch);
        }
        builder
    }

    /// PSUBSCRIBE pattern [pattern ...] — Subscribe by pattern
    #[must_use = "call .build() to encode the command"]
    fn psubscribe<K: ToRedisArgs>(&self, patterns: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("PSUBSCRIBE");
        for p in patterns {
            builder = builder.arg(p);
        }
        builder
    }

    /// PUNSUBSCRIBE — Unsubscribe from all patterns
    #[must_use = "call .build() to encode the command"]
    fn punsubscribe(&self) -> CommandBuilder {
        CommandBuilder::new("PUNSUBSCRIBE")
    }

    /// PUNSUBSCRIBE pattern [pattern ...] — Unsubscribe from specific patterns
    #[must_use = "call .build() to encode the command"]
    fn punsubscribe_patterns<K: ToRedisArgs>(&self, patterns: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("PUNSUBSCRIBE");
        for p in patterns {
            builder = builder.arg(p);
        }
        builder
    }
}
