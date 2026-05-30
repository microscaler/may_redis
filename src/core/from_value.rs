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

impl FromRedisValue for Vec<Option<String>> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(arr) => {
                let mut result = Self::with_capacity(arr.len());
                for element in arr {
                    result.push(Option::<String>::from_redis_value(element)?);
                }
                Ok(result)
            }
            other => Err(RedisError::Parse(format!(
                "expected Array for Vec<Option<String>>, got {other:?}"
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
            RedisValue::Integer(n) if *n >= 0 => {
                let n = *n as u64;
                u64::try_into(n)
                    .map_err(|_| RedisError::Parse(format!("integer {n} is too large for usize")))
            }
            RedisValue::Integer(n) => Err(RedisError::Parse(format!(
                "negative integer {n} cannot be converted to usize"
            ))),
            other => Err(RedisError::Parse(format!(
                "expected Integer for usize, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for u64 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) if *n >= 0 => Ok(*n as Self),
            RedisValue::Integer(n) => Err(RedisError::Parse(format!(
                "negative integer {n} cannot be converted to u64"
            ))),
            other => Err(RedisError::Parse(format!(
                "expected Integer for u64, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for i32 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) => (i64::from(*n as Self) == *n)
                .then_some(*n as Self)
                .ok_or_else(|| RedisError::Parse(format!("integer {n} is out of range for i32"))),
            other => Err(RedisError::Parse(format!(
                "expected Integer for i32, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for u8 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) if i64::from(*n as Self) == *n && *n >= 0 => Ok(*n as Self),
            RedisValue::Integer(n) => Err(RedisError::Parse(format!(
                "integer {n} is out of range for u8"
            ))),
            other => Err(RedisError::Parse(format!(
                "expected Integer for u8, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for f64 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::BulkString(b) => std::str::from_utf8(b)
                .map_err(|_| RedisError::Parse("BulkString is not valid UTF-8".to_string()))
                .and_then(|s| {
                    s.parse::<Self>()
                        .map_err(|e| RedisError::Parse(format!("cannot parse '{s}' as f64: {e}")))
                }),
            other => Err(RedisError::Parse(format!(
                "expected BulkString for f64, got {other:?}"
            ))),
        }
    }
}
