// Commands — transactions trait
//
// Provides all Transactions commands for Redis data structures.

use crate::core::ToRedisArgs;

use super::CommandBuilder;

/// Trait providing Transactions command methods.
pub trait TransactionsCommands: Sized {

    /// MULTI — Start a transaction
    #[must_use = "call .build() to encode the command"]
    fn multi(&self) -> CommandBuilder {
        CommandBuilder::new("MULTI")
    }


    /// EXEC — Execute the transaction
    #[must_use = "call .build() to encode the command"]
    fn exec(&self) -> CommandBuilder {
        CommandBuilder::new("EXEC")
    }


    /// DISCARD — Abort the transaction
    #[must_use = "call .build() to encode the command"]
    fn discard(&self) -> CommandBuilder {
        CommandBuilder::new("DISCARD")
    }


    /// WATCH key [key ...] — Monitor keys for transactional changes
    #[must_use = "call .build() to encode the command"]
    fn watch<K: ToRedisArgs>(&self, keys: &[K]) -> CommandBuilder {
        let mut builder = CommandBuilder::new("WATCH");
        for key in keys {
            builder = builder.arg(key);
        }
        builder
    }


    /// UNWATCH — Clear all watched keys
    #[must_use = "call .build() to encode the command"]
    fn unwatch(&self) -> CommandBuilder {
        CommandBuilder::new("UNWATCH")
    }

}
