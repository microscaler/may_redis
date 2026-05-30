/// Protocol for building and encoding Redis commands.
///
/// The `Commands` trait provides all Redis command methods, each constructing
/// a `CommandBuilder` for RESP2 wire format.

mod strings;
mod hashes;
mod sets;
mod lists;
mod sorted_sets;
mod pubsub;
mod transactions;
mod admin;

pub use strings::StringsCommands;
pub use hashes::HashesCommands;
pub use sets::SetsCommands;
pub use lists::ListsCommands;
pub use sorted_sets::SortedSetsCommands;
pub use pubsub::PubsubCommands;
pub use transactions::TransactionsCommands;
pub use admin::AdminCommands;

// Re-export CommandBuilder for use in domain modules
pub use super::builder::CommandBuilder;

/// Trait that provides all Redis command methods.
///
/// Each method constructs a `CommandBuilder` for a specific Redis command,
/// which can then be encoded into RESP2 wire format via [`build()`](CommandBuilder::build).
pub trait Commands: Sized
    + StringsCommands
    + HashesCommands
    + SetsCommands
    + ListsCommands
    + SortedSetsCommands
    + PubsubCommands
    + TransactionsCommands
    + AdminCommands
{
}

// Blanket impl: any type implementing all domain traits automatically implements Commands
impl<T: StringsCommands + HashesCommands + SetsCommands + ListsCommands + SortedSetsCommands + PubsubCommands + TransactionsCommands + AdminCommands> Commands for T {}

// Empty impls for () so it can use all commands (all methods have default implementations)
impl StringsCommands for () {}
impl HashesCommands for () {}
impl SetsCommands for () {}
impl ListsCommands for () {}
impl SortedSetsCommands for () {}
impl PubsubCommands for () {}
impl TransactionsCommands for () {}
impl AdminCommands for () {}

#[cfg(test)]
mod admin_tests;
#[cfg(test)]
mod hashes_tests;
#[cfg(test)]
mod lists_tests;
#[cfg(test)]
mod pubsub_tests;
#[cfg(test)]
mod sets_tests;
#[cfg(test)]
mod sorted_sets_tests;
#[cfg(test)]
mod strings_tests;
#[cfg(test)]
mod transactions_tests;
