// RedisError — Error types for may_redis.
//
// All errors from may_redis are captured by this enum. It maps naturally to
// the Redis error hierarchy: connection failures, protocol violations, parse
// errors, and generic fallbacks.

use std::fmt;

use super::value::RedisValue;

/// A typed error from `may_redis` operations.
///
/// Every error from the base layer is an instance of this enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedisError {
    /// A connection error (e.g. TCP refused, timeout, reset).
    Connection(String),
    /// A protocol error (e.g. malformed RESP, unexpected response type).
    Protocol(String),
    /// A parse error (e.g. invalid UTF-8, conversion failure).
    Parse(String),
    /// A security policy violation (e.g. command denied by `CommandPolicy`).
    Security(String),
    /// A generic error that does not fit the above categories.
    Other(String),
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "connection: {msg}"),
            Self::Protocol(msg) => write!(f, "protocol: {msg}"),
            Self::Parse(msg) => write!(f, "parse: {msg}"),
            Self::Security(msg) => write!(f, "security: {msg}"),
            Self::Other(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl std::error::Error for RedisError {}

impl From<String> for RedisError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for RedisError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}

impl From<RedisValue> for RedisError {
    fn from(value: RedisValue) -> Self {
        match value {
            RedisValue::Error(msg) => Self::Protocol(msg),
            other => Self::Protocol(format!("unexpected value: {other:?}")),
        }
    }
}

/// Convenience alias for `Result<T, RedisError>`.
pub type RedisResult<T> = Result<T, RedisError>;

/// Trait for converting a [`RedisValue`] into a Rust type.
///
/// Every type that can be extracted from a Redis response must implement
/// this trait. It is the inverse of [`ToRedisArgs`].
///
/// [`ToRedisArgs`]: crate::ToRedisArgs
///
/// # Errors
/// Returns [`RedisError::Parse`] if the `RedisValue` cannot be converted
/// to the requested Rust type.
pub trait FromRedisValue: Sized {
    /// Convert a `&RedisValue` into `Self`, returning an error on type mismatch.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the `RedisValue` does not match the
    /// expected type for the target Rust type (e.g. `Integer` expected for
    /// `i64`, `BulkString` expected for `String`).
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self>;
}

// ---------------------------------------------------------------------------
// FromRedisValue implementations
// ---------------------------------------------------------------------------

impl FromRedisValue for i64 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) => Ok(*n),
            other => Err(RedisError::Parse(format!(
                "expected Integer, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for String {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::BulkString(bytes) => std::str::from_utf8(bytes)
                .map(ToString::to_string)
                .map_err(|_| RedisError::Parse("BulkString is not valid UTF-8".to_string())),
            RedisValue::SimpleString(s) => Ok(s.clone()),
            RedisValue::Integer(n) => Ok(n.to_string()),
            other => Err(RedisError::Parse(format!(
                "expected BulkString, SimpleString, or Integer, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for () {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::SimpleString(s) if s == "OK" => Ok(()),
            RedisValue::Integer(1 | 0) => Ok(()),
            other => Err(RedisError::Parse(format!(
                "expected OK, Integer(0), or Integer(1), got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for bool {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(0) => Ok(false),
            RedisValue::Integer(1) => Ok(true),
            other => Err(RedisError::Parse(format!(
                "expected Integer(0) or Integer(1), got {other:?}"
            ))),
        }
    }
}
