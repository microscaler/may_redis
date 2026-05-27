// RedisValue — Canonical representation of a Redis value.
//
// Mirrors the Redis wire format types in RESP2:
// - BulkString: a UTF-8 string with associated bytes
// - Array: a nested collection of RedisValue
// - Integer: a signed 64-bit integer (used for INCR, EXISTS, etc.)
// - SimpleString: the "OK" response type
// - Error: the "-ERR ..." response type
// - Null: the null bulk string response

/// The canonical representation of a Redis value.
///
/// This enum is the central data type in may-redis. All data flowing through
/// the codec, protocol, connection, and client layers is represented as
/// `RedisValue`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum RedisValue {
    /// A bulk string: `$N\r\n...\r\n` in RESP2.
    BulkString(Vec<u8>),
    /// An array: `*N\r\n...` in RESP2, where each element is itself a `RedisValue`.
    Array(Vec<Self>),
    /// An integer response: `:N\r\n` in RESP2.
    Integer(i64),
    /// A simple string: `+OK\r\n` in RESP2.
    SimpleString(String),
    /// An error response: `-ERR message\r\n` in RESP2.
    Error(String),
    /// A null response: `$-1\r\n` in RESP2.
    #[default]
    Null,
}

impl RedisValue {
    /// Returns `true` if this value is [`Null`].
    ///
    /// [`Null`]: RedisValue::Null
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns `true` if this value is an [`Error`].
    ///
    /// [`Error`]: RedisValue::Error
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns `true` if this value is an [`Integer`].
    ///
    /// [`Integer`]: RedisValue::Integer
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    /// Extract the inner `i64` if this is an [`Integer`], or `None` otherwise.
    ///
    /// [`Integer`]: RedisValue::Integer
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract the inner `&str` if this is a [`SimpleString`], [`BulkString`],
    /// or [`Error`] (assuming valid UTF-8), or `None` otherwise.
    ///
    /// [`SimpleString`]: RedisValue::SimpleString
    /// [`BulkString`]: RedisValue::BulkString
    /// [`Error`]: RedisValue::Error
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::BulkString(b) => std::str::from_utf8(b).ok(),
            Self::SimpleString(s) | Self::Error(s) => Some(s),
            _ => None,
        }
    }

    /// Extract the inner `&[u8]` if this is a [`BulkString`], or `None` otherwise.
    ///
    /// [`BulkString`]: RedisValue::BulkString
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::BulkString(b) => Some(b),
            _ => None,
        }
    }

    /// Extract the inner `&[Self]` if this is an [`Array`], or `None` otherwise.
    ///
    /// [`Array`]: RedisValue::Array
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_value_integer_variant() {
        let val = RedisValue::Integer(42);
        assert!(val.is_integer());
        assert_eq!(val.as_integer(), Some(42));
        assert!(val.as_bytes().is_none());
        assert!(val.as_array().is_none());
    }

    #[test]
    fn test_redis_value_bulk_string_variant() {
        let val = RedisValue::BulkString(b"hello".to_vec());
        assert_eq!(val.as_bytes(), Some(b"hello".as_slice()));
        assert_eq!(val.as_str(), Some("hello"));
        assert!(!val.is_null());
        assert!(!val.is_error());
    }

    #[test]
    fn test_redis_value_array_variant() {
        let arr = vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(b"test".to_vec()),
        ];
        let val = RedisValue::Array(arr.clone());
        assert_eq!(val.as_array(), Some(arr.as_slice()));
        assert_eq!(val.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_redis_value_is_null() {
        let null_val = RedisValue::Null;
        assert!(null_val.is_null());
        assert!(!null_val.is_integer());
        assert!(!null_val.is_error());
    }

    #[test]
    fn test_redis_value_clone() {
        let val = RedisValue::BulkString(b"hello".to_vec());
        let cloned = val.clone();
        assert_eq!(val, cloned);

        // Deep clone for nested arrays
        let nested = RedisValue::Array(vec![RedisValue::Integer(42)]);
        let cloned_nested = nested.clone();
        assert_eq!(nested, cloned_nested);
    }

    #[test]
    fn test_redis_value_default() {
        let val = RedisValue::default();
        assert!(matches!(val, RedisValue::Null));
    }

    #[test]
    fn test_redis_value_simple_string() {
        let val = RedisValue::SimpleString("OK".to_string());
        assert_eq!(val.as_str(), Some("OK"));
        assert!(!val.is_error());
    }

    #[test]
    fn test_redis_value_error() {
        let val = RedisValue::Error("ERR key not found".to_string());
        assert!(val.is_error());
        assert_eq!(val.as_str(), Some("ERR key not found"));
    }
}
