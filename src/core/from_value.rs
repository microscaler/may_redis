// Additional FromRedisValue implementations for types used by the Sesame-IDAM
// Redis command set.

use super::error::RedisResult;
use super::{FromRedisValue, RedisError, RedisValue};

impl FromRedisValue for Vec<String> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(arr) => {
                let mut result = Self::with_capacity(arr.len());
                for element in arr {
                    result.push(String::from_redis_value(element)?);
                }
                Ok(result)
            }
            other => Err(RedisError::Parse(format!(
                "expected Array for Vec<String>, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for Vec<i64> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(arr) => {
                let mut result = Self::with_capacity(arr.len());
                for element in arr {
                    result.push(i64::from_redis_value(element)?);
                }
                Ok(result)
            }
            other => Err(RedisError::Parse(format!(
                "expected Array for Vec<i64>, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for Vec<RedisValue> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(arr) => Ok(arr.clone()),
            other => Err(RedisError::Parse(format!(
                "expected Array for Vec<RedisValue>, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for Option<String> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Null => Ok(None),
            RedisValue::BulkString(bytes) => std::str::from_utf8(bytes)
                .map(ToString::to_string)
                .map(Some)
                .map_err(|_| RedisError::Parse("BulkString is not valid UTF-8".to_string())),
            RedisValue::SimpleString(s) => Ok(Some(s.clone())),
            other => Err(RedisError::Parse(format!(
                "expected Null, BulkString, or SimpleString for Option<String>, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for usize {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) if *n >= 0 => Ok(*n as Self),
            RedisValue::Integer(n) => Err(RedisError::Parse(format!(
                "negative integer {n} cannot be converted to usize"
            ))),
            other => Err(RedisError::Parse(format!(
                "expected Integer for usize, got {other:?}"
            ))),
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_redis_value_array_to_vec_string() {
        let arr = vec![
            RedisValue::BulkString(b"foo".to_vec()),
            RedisValue::BulkString(b"bar".to_vec()),
        ];
        let val = RedisValue::Array(arr);
        let result: Vec<String> = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn test_from_redis_value_array_to_vec_i64() {
        let arr = vec![
            RedisValue::Integer(1),
            RedisValue::Integer(2),
            RedisValue::Integer(3),
        ];
        let val = RedisValue::Array(arr);
        let result: Vec<i64> = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_from_redis_value_null_to_option_string_none() {
        let val = RedisValue::Null;
        let result: Option<String> = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_from_redis_value_bulk_string_to_option_string_some() {
        let val = RedisValue::BulkString(b"hello".to_vec());
        let result: Option<String> = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_from_redis_value_simple_string_to_option_string_some() {
        let val = RedisValue::SimpleString("OK".to_string());
        let result: Option<String> = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, Some("OK".to_string()));
    }

    #[test]
    fn test_from_redis_value_array_to_vec_redis_value() {
        let arr = vec![
            RedisValue::Integer(42),
            RedisValue::BulkString(b"test".to_vec()),
        ];
        let val = RedisValue::Array(arr);
        let result: Vec<RedisValue> = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], RedisValue::Integer(42));
        assert_eq!(result[1], RedisValue::BulkString(b"test".to_vec()));
    }

    #[test]
    fn test_from_redis_value_usize() {
        let val = RedisValue::Integer(42);
        let result: usize = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_from_redis_value_usize_negative_error() {
        let val = RedisValue::Integer(-1);
        let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_vec_string_wrong_type() {
        let val = RedisValue::BulkString(b"not an array".to_vec());
        let result: Result<Vec<String>, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_empty_array() {
        let val = RedisValue::Array(vec![]);
        let result: Vec<String> = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(result.is_empty());
    }
}
