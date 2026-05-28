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
            RedisValue::Integer(n) if *n >= 0 => Ok(*n as u64),
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
            RedisValue::Integer(n) => (*n as i32 as i64 == *n)
                .then_some(*n as i32)
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
            RedisValue::Integer(n) if (*n as u8 as i64) == *n && *n >= 0 => Ok(*n as u8),
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
                    s.parse::<f64>()
                        .map_err(|e| RedisError::Parse(format!("cannot parse '{s}' as f64: {e}")))
                }),
            other => Err(RedisError::Parse(format!(
                "expected BulkString for f64, got {other:?}"
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

    // --- Story 12 tests (T1: usize upper-bound check) ---

    #[test]
    fn test_from_redis_value_usize_zero() {
        let val = RedisValue::Integer(0);
        let result: usize = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_from_redis_value_usize_positive() {
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
    fn test_from_redis_value_usize_i64_max() {
        // i64::MAX is 9223372036854775807
        // On 64-bit: usize::MAX = 18446744073709551615, so i64::MAX fits
        // On 32-bit: usize::MAX = 4294967295, so i64::MAX overflows
        let val = RedisValue::Integer(i64::MAX);
        let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
        // On 64-bit this succeeds, on 32-bit it fails
        if cfg!(target_pointer_width = "64") {
            assert_eq!(result.unwrap() as i64, i64::MAX);
        } else {
            assert!(result.is_err());
        }
    }

    // --- Story 13 tests (T2: missing FromRedisValue impls) ---

    #[test]
    fn test_from_redis_value_u64_zero() {
        let val = RedisValue::Integer(0);
        let result: u64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_from_redis_value_u64_max() {
        let val = RedisValue::Integer(i64::MAX);
        let result: u64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i64::MAX as u64);
    }

    #[test]
    fn test_from_redis_value_u64_negative_error() {
        let val = RedisValue::Integer(-1);
        let result: Result<u64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_u64_wrong_type() {
        let val = RedisValue::BulkString(b"not an int".to_vec());
        let result: Result<u64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i32_zero() {
        let val = RedisValue::Integer(0);
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_from_redis_value_i32_max() {
        let val = RedisValue::Integer(i32::MAX as i64);
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i32::MAX);
    }

    #[test]
    fn test_from_redis_value_i32_min() {
        let val = RedisValue::Integer(i32::MIN as i64);
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i32::MIN);
    }

    #[test]
    fn test_from_redis_value_i32_overflow() {
        let val = RedisValue::Integer(i64::MAX);
        let result: Result<i32, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_u8_zero() {
        let val = RedisValue::Integer(0);
        let result: u8 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_from_redis_value_u8_max() {
        let val = RedisValue::Integer(255);
        let result: u8 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 255);
    }

    #[test]
    fn test_from_redis_value_u8_overflow() {
        let val = RedisValue::Integer(256);
        let result: Result<u8, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_u8_negative_error() {
        let val = RedisValue::Integer(-1);
        let result: Result<u8, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_basic() {
        let val = RedisValue::BulkString(b"3.14".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_negative() {
        let val = RedisValue::BulkString(b"-1.5".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - (-1.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_scientific() {
        let val = RedisValue::BulkString(b"1.5e10".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - 1.5e10).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_zero() {
        let val = RedisValue::BulkString(b"0.0".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_from_redis_value_f64_wrong_type() {
        let val = RedisValue::Integer(42);
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_non_utf8_error() {
        let val = RedisValue::BulkString(vec![0xff, 0xfe]);
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_parse_error() {
        let val = RedisValue::BulkString(b"not a number".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }
}
