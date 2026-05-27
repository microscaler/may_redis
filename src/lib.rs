// may-redis — A coroutine-native Redis client for the may runtime
//
// Zero tokio, zero async-await, only may coroutines.
//
// Module layout:
// - base:        RedisValue, RedisError, FromRedisValue, ToRedisArgs
// - codec:       RESP encoding/decoding (writer + reader)
// - protocol:    CommandBuilder, Commands trait
// - connection:  epoll connection loop, TCP, coroutine management
// - client:      RedisClient, Pipeline, public API

pub mod base;
pub mod codec;
pub mod protocol;
pub mod connection;
pub mod client;
