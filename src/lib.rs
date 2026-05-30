// may-redis — A coroutine-native Redis client for the may runtime
//
// Zero tokio, zero async-await, only may coroutines.
//
// Module layout:
// - core:        RedisValue, RedisError, FromRedisValue, ToRedisArgs
// - codec:       RESP encoding/decoding (writer + reader)
// - protocol:    CommandBuilder, Commands trait
// - connection:  epoll connection loop, TCP, coroutine management
// - client:      RedisClient, Pipeline, public API

#![allow(clippy::doc_markdown)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::transmute_ptr_to_ref)]
#![allow(clippy::io_other_error)]
#![allow(clippy::ref_as_ptr)]

pub mod client;
pub mod codec;
pub mod connection;
pub mod core;
pub mod protocol;
#[cfg(feature = "tls")]
pub mod tls;

// Re-export the most common types at the crate root
pub use client::client::RedisClient;
pub use client::pipeline::Pipeline;
pub use client::pipeline_response::FromPipelineResponse;
pub use core::{FromRedisValue, RedisError, RedisValue, ToRedisArgs};
pub use protocol::builder::{cmd, CommandBuilder, CommandPolicy};
pub use protocol::commands::Commands;

#[cfg(feature = "test")]
pub use client::in_memory::InMemoryClient;
