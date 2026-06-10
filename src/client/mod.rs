// client — Public client API
//
// Provides the user-facing API:
// - RedisClient: main entry point for all Redis operations
// - Pipeline: batch command execution
// - InMemoryClient: test-only in-memory backend
// - Mirrors the redis crate API surface for mechanical migration

//! # client
//!
//! Public client API: `RedisClient`, `Pipeline`, `InMemoryClient`.
//!
//! This crate assembles all lower-level crates: base, codec, protocol, connection.
//!
//! ## Architecture
//!
//! ```mermaid
//! graph TB
//!     RC[RedisClient] --> CT[Commands trait]
//!     RC --> PJ[Pipeline]
//!     RC --> IC[InMemoryClient]
//!     RC --> CO[Connection]
//!     CO --> Proto[protocol]
//!     CO --> Codec[codec]
//!     CO --> Base[base]
//!     PJ --> Proto
//!     PJ --> Codec
//!     PJ --> Base
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use may_redis::RedisClient;
//!
//! let client = RedisClient::connect("127.0.0.1", 6379).unwrap();
//! ```
//!

pub use client::RedisClient;
pub use pipeline::Pipeline;
pub use pipeline_response::FromPipelineResponse;
pub use pubsub_client::PubSubClient;

#[cfg(feature = "test")]
pub use in_memory::InMemoryClient;

#[cfg(feature = "test")]
pub mod in_memory;

#[allow(clippy::module_inception)]
pub mod client;
mod client_timeout;
mod client_url;
pub mod pipeline;
pub mod pipeline_response;
pub mod pubsub_client;

#[cfg(test)]
mod client_tests {
    #[cfg(test)]
    mod execute_errors;
    #[cfg(test)]
    mod integration;
    #[cfg(test)]
    mod integration_admin_advanced;
    #[cfg(test)]
    mod integration_admin_basic;
    #[cfg(test)]
    mod integration_hashes_advanced;
    #[cfg(test)]
    mod integration_hashes_basic;
    #[cfg(test)]
    mod integration_lists_basic;
    #[cfg(test)]
    mod integration_pubsub;
    #[cfg(test)]
    mod integration_sets_basic;
    #[cfg(test)]
    mod integration_sorted_sets;
    #[cfg(test)]
    mod integration_strings_advanced;
    #[cfg(test)]
    mod integration_strings_basic;
    #[cfg(test)]
    mod integration_timeout;
    #[cfg(test)]
    mod integration_transactions;
    #[cfg(test)]
    mod pipeline_errors;
    #[cfg(test)]
    mod tls_tests;
    #[cfg(test)]
    mod unit;
}
