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

impl FromRedisValue for Option<Vec<String>> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Null => Ok(None),
            other => Vec::<String>::from_redis_value(other).map(Some),
        }
    }
}

impl FromRedisValue for usize {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) if *n >= 0 => {
                let n = *n as u64;
                u64::try_into(n).map_err(|_| {
                    RedisError::Parse(format!("integer {n} is too large for usize"))
                })
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
                .ok_or_else(|| {
                    RedisError::Parse(format!("integer {n} is out of range for i32"))
                }),
            other => Err(RedisError::Parse(format!(
                "expected Integer for i32, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for u8 {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Integer(n) if i64::from(*n as Self) == *n && *n >= 0 => {
                Ok(*n as Self)
            }
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
                .map_err(|_| {
                    RedisError::Parse("BulkString is not valid UTF-8".to_string())
                })
                .and_then(|s| {
                    s.parse::<Self>().map_err(|e| {
                        RedisError::Parse(format!("cannot parse '{s}' as f64: {e}"))
                    })
                }),
            other => Err(RedisError::Parse(format!(
                "expected BulkString for f64, got {other:?}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Compound types — collections of key/value pairs and scan cursors
// ---------------------------------------------------------------------------

/// Parse a SCAN-family cursor (Redis RESP2 sends bulk strings, e.g. `"0"`).
fn scan_cursor(value: &RedisValue) -> RedisResult<i64> {
    match value {
        RedisValue::Integer(n) => Ok(*n),
        RedisValue::BulkString(bytes) => {
            let s = std::str::from_utf8(bytes).map_err(|_| {
                RedisError::Parse("scan cursor is not valid UTF-8".into())
            })?;
            s.parse::<i64>().map_err(|_| {
                RedisError::Parse(format!("cannot parse scan cursor from '{s}'"))
            })
        }
        RedisValue::SimpleString(s) => s.parse::<i64>().map_err(|_| {
            RedisError::Parse(format!("cannot parse scan cursor from '{s}'"))
        }),
        other => Err(RedisError::Parse(format!(
            "expected scan cursor (bulk string or integer), got {other:?}"
        ))),
    }
}

impl FromRedisValue for Vec<(String, String)> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(items) => {
                if items.len() % 2 != 0 {
                    return Err(RedisError::Parse(format!(
                        "expected even-length array of key/value pairs, \
                         got odd length {}",
                        items.len()
                    )));
                }
                let mut result = Self::with_capacity(items.len());
                let mut iter = items.iter();
                while let (Some(key), Some(val)) = (iter.next(), iter.next()) {
                    result.push((
                        String::from_redis_value(key)?,
                        String::from_redis_value(val)?,
                    ));
                }
                Ok(result)
            }
            other => Err(RedisError::Parse(format!(
                "expected Array (alternating bulk strings), got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for Vec<(String, f64)> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(items) => {
                if items.len() % 2 != 0 {
                    return Err(RedisError::Parse(format!(
                        "expected even-length array of member/score pairs, \
                         got odd length {}",
                        items.len()
                    )));
                }
                let mut result = Self::with_capacity(items.len());
                let mut iter = items.iter();
                while let (Some(key), Some(score)) = (iter.next(), iter.next()) {
                    let score_str = String::from_redis_value(score)?;
                    let score_val: f64 = score_str.parse().map_err(|_| {
                        RedisError::Parse(format!(
                            "cannot parse score from {score_str}"
                        ))
                    })?;
                    result.push((String::from_redis_value(key)?, score_val));
                }
                Ok(result)
            }
            other => Err(RedisError::Parse(format!(
                "expected Array (alternating bulk string and score), got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for (i64, Vec<String>) {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(items) => {
                if items.len() < 2 {
                    return Err(RedisError::Parse(format!(
                        "expected at least 2 items (cursor + array), got {:?}",
                        items.len()
                    )));
                }
                let cursor = scan_cursor(&items[0])?;
                let values = Vec::<String>::from_redis_value(&items[1])?;
                Ok((cursor, values))
            }
            other => Err(RedisError::Parse(format!("expected Array, got {other:?}"))),
        }
    }
}

impl FromRedisValue for (i64, Vec<(String, f64)>) {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Array(items) => {
                if items.len() < 2 {
                    return Err(RedisError::Parse(format!(
                        "expected at least 2 items (cursor + array), got {:?}",
                        items.len()
                    )));
                }
                let cursor = scan_cursor(&items[0])?;
                let members = Vec::<(String, f64)>::from_redis_value(&items[1])?;
                Ok((cursor, members))
            }
            other => Err(RedisError::Parse(format!("expected Array, got {other:?}"))),
        }
    }
}

impl FromRedisValue for Option<i64> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Null => Ok(None),
            RedisValue::Integer(n) => Ok(Some(*n)),
            other => Err(RedisError::Parse(format!(
                "expected Null or Integer for Option<i64>, got {other:?}"
            ))),
        }
    }
}

impl FromRedisValue for Option<f64> {
    fn from_redis_value(value: &RedisValue) -> RedisResult<Self> {
        match value {
            RedisValue::Null => Ok(None),
            RedisValue::BulkString(b) => std::str::from_utf8(b)
                .map_err(|_| {
                    RedisError::Parse("BulkString is not valid UTF-8".to_string())
                })
                .and_then(|s| {
                    s.trim().parse::<f64>().map(Some).map_err(|_| {
                        RedisError::Parse(format!("cannot parse '{s}' as f64"))
                    })
                }),
            other => Err(RedisError::Parse(format!(
                "expected Null or BulkString for Option<f64>, got {other:?}"
            ))),
        }
    }
}
