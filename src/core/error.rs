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
    /// A generic error that does not fit the above categories.
    Other(String),
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "connection: {msg}"),
            Self::Protocol(msg) => write!(f, "protocol: {msg}"),
            Self::Parse(msg) => write!(f, "parse: {msg}"),
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

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_error_display() {
        let e = RedisError::Connection("refused".to_string());
        assert_eq!(format!("{e}"), "connection: refused");

        let e = RedisError::Protocol("bad response".to_string());
        assert_eq!(format!("{e}"), "protocol: bad response");

        let e = RedisError::Parse("invalid UTF-8".to_string());
        assert_eq!(format!("{e}"), "parse: invalid UTF-8");

        let e = RedisError::Other("unknown".to_string());
        assert_eq!(format!("{e}"), "error: unknown");
    }

    #[test]
    fn test_redis_error_from_string() {
        let e: RedisError = "boom".to_string().into();
        assert!(matches!(e, RedisError::Other(_)));
    }

    #[test]
    fn test_from_redis_value_integer_to_i64() {
        let val = RedisValue::Integer(42);
        let n: i64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_from_redis_value_integer_to_i64_wrong_type() {
        let val = RedisValue::BulkString(b"not an int".to_vec());
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_bulk_string_to_string() {
        let val = RedisValue::BulkString(b"hello".to_vec());
        let s: String = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_from_redis_value_simple_string_to_string() {
        let val = RedisValue::SimpleString("OK".to_string());
        let s: String = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(s, "OK");
    }

    #[test]
    fn test_from_redis_value_to_unit_ok() {
        let val = RedisValue::Integer(1);
        let result: () = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_from_redis_value_to_bool_true() {
        let val = RedisValue::Integer(1);
        let b: bool = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(b);
    }

    #[test]
    fn test_from_redis_value_to_bool_false() {
        let val = RedisValue::Integer(0);
        let b: bool = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(!b);
    }

    #[test]
    fn test_from_redis_value_null_to_string() {
        let val = RedisValue::Null;
        let result: Result<String, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------------------
    // i64 coverage tests — verify rejection of all non-Integer types
    // ---------------------------------------------------------------------------

    #[test]
    fn test_from_redis_value_i64_bulk_numeric_rejected() {
        let val = RedisValue::BulkString(b"42".to_vec());
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("expected Integer"));
    }

    #[test]
    fn test_from_redis_value_i64_simple_numeric_rejected() {
        let val = RedisValue::SimpleString("42".to_string());
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i64_null_rejected() {
        let val = RedisValue::Null;
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i64_error_rejected() {
        let val = RedisValue::Error("ERR msg".to_string());
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i64_array_rejected() {
        let val = RedisValue::Array(vec![RedisValue::Integer(1)]);
        let result: Result<i64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i64_zero() {
        let val = RedisValue::Integer(0);
        let n: i64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_from_redis_value_i64_min() {
        let val = RedisValue::Integer(i64::MIN);
        let n: i64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(n, i64::MIN);
    }

    #[test]
    fn test_from_redis_value_i64_max() {
        let val = RedisValue::Integer(i64::MAX);
        let n: i64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(n, i64::MAX);
    }

    // ---------------------------------------------------------------------------
    // String from Integer tests — Story 8.7
    // ---------------------------------------------------------------------------

    #[test]
    fn test_from_redis_value_string_from_int() {
        let val = RedisValue::Integer(42);
        let s: String = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn test_from_redis_value_string_from_int_negative() {
        let val = RedisValue::Integer(-1);
        let s: String = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(s, "-1");
    }

    #[test]
    fn test_from_redis_value_string_from_int_max() {
        let val = RedisValue::Integer(i64::MAX);
        let s: String = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(s, i64::MAX.to_string());
    }

    #[test]
    fn test_from_redis_value_string_from_int_error() {
        let val = RedisValue::Error("ERR msg".to_string());
        let result: Result<String, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_string_from_int_null() {
        let val = RedisValue::Null;
        let result: Result<String, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_string_from_int_array() {
        let val = RedisValue::Array(vec![RedisValue::Integer(1)]);
        let result: Result<String, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------------------
    // () with Integer(0) test — Story 8.6
    // ---------------------------------------------------------------------------

    #[test]
    fn test_from_redis_value_unit_int_zero() {
        let val = RedisValue::Integer(0);
        let result: () = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_from_redis_value_unit_error_rejected() {
        let val = RedisValue::Error("ERR msg".to_string());
        let result: Result<(), _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_unit_null_rejected() {
        let val = RedisValue::Null;
        let result: Result<(), _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_unit_bulk_rejected() {
        let val = RedisValue::BulkString(b"nope".to_vec());
        let result: Result<(), _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }
}
