// FromRedisValue impl tests for Vec<T>, Option<String>, usize, u64, i32, u8, f64
//
// Tests all FromRedisValue trait implementations added across Stories 5-13.

use crate::core::FromRedisValue;
use crate::core::RedisValue;

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
    fn test_from_redis_value_usize_i64_max() {
        // i64::MAX is 9223372036854775807
        // On 64-bit: usize::MAX = 18446744073709551615, so i64::MAX fits
        // On 32-bit: usize::MAX = 4294967295, so i64::MAX overflows
        let val = RedisValue::Integer(i64::MAX);
        let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
        // On 64-bit this succeeds, on 32-bit it fails
        if cfg!(target_pointer_width = "64") {
            #[allow(clippy::cast_possible_wrap)]
            {
                assert_eq!(result.unwrap() as i64, i64::MAX);
            }
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
        let val = RedisValue::Integer(i64::from(i32::MAX));
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i32::MAX);
    }

    #[test]
    fn test_from_redis_value_i32_min() {
        let val = RedisValue::Integer(i64::from(i32::MIN));
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
        let val = RedisValue::BulkString(b"1.23".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - 1.23).abs() < f64::EPSILON);
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
        assert!(result.abs() < f64::EPSILON);
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

    // --- Story 12.7: f64 edge cases (Redis infinity, NaN, empty, exponents) ---

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_from_redis_value_f64_inf() {
        // Redis returns "inf" (not "Infinity") for positive infinity
        let val = RedisValue::BulkString(b"inf".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, f64::INFINITY);
    }

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_from_redis_value_f64_neg_inf() {
        let val = RedisValue::BulkString(b"-inf".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, f64::NEG_INFINITY);
    }

    #[test]
    fn test_from_redis_value_f64_nan() {
        // Redis returns lowercase "nan"
        let val = RedisValue::BulkString(b"nan".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(result.is_nan());
    }

    #[test]
    fn test_from_redis_value_f64_empty() {
        let val = RedisValue::BulkString(b"".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_whitespace() {
        let val = RedisValue::BulkString(b"  ".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_trailing_garbage() {
        let val = RedisValue::BulkString(b"3.14abc".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_multiple_decimals() {
        let val = RedisValue::BulkString(b"1.2.3".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_decimal_only() {
        let val = RedisValue::BulkString(b".".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_neg_zero() {
        let val = RedisValue::BulkString(b"-0.0".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(result.is_sign_negative());
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_exp_neg() {
        let val = RedisValue::BulkString(b"1e-10".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - 1e-10).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_underscores() {
        // Rust's f64::from_str does NOT support underscores in float strings
        // (only integer literals have underscore support in source code)
        let val = RedisValue::BulkString(b"1_000.5".to_vec());
        let result: Result<f64, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_f64_exp_sign() {
        // Explicit positive sign is supported by Rust's f64::from_str
        let val = RedisValue::BulkString(b"+1.5".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_near_max() {
        // 1e308 is near DBL_MAX (~1.8e308) and should parse to a large finite number
        let val = RedisValue::BulkString(b"1e308".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(!result.is_infinite());
        assert!(!result.is_nan());
        assert!(result.is_finite());
        assert!((result - 1e308).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_redis_value_f64_near_min() {
        // 1e-308 is near DBL_MIN (~2.2e-308) — a valid small positive number
        // that does not underflow to zero.
        let val = RedisValue::BulkString(b"1e-308".to_vec());
        let result: f64 = FromRedisValue::from_redis_value(&val).unwrap();
        assert!(!result.is_infinite());
        assert!(!result.is_nan());
        assert!(result.is_finite());
        assert!(result > 0.0);
    }

    // --- Story 5: usize exact boundary tests ---

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_from_redis_value_usize_exact_max() {
        // usize::MAX as an Integer should parse successfully.
        // On 32-bit: usize::MAX = 4294967295, fits in i64.
        // On 64-bit: usize::MAX = 18446744073709551615, does NOT fit in i64
        // and usize::MAX as i64 would truncate to -1. Guarded for 32-bit only.
        if cfg!(target_pointer_width = "32") {
            let val = RedisValue::Integer(usize::MAX as i64);
            let result: usize = FromRedisValue::from_redis_value(&val).unwrap();
            assert_eq!(result, usize::MAX);
        }
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_from_redis_value_usize_overflow_boundary() {
        // usize::MAX + 1 should return an error.
        // On 64-bit, usize::MAX > i64::MAX, so we cannot create a RedisValue::Integer
        // with value usize::MAX + 1 (it would exceed i64::MAX). The boundary is
        // already covered by the i64::MAX test. This test is only meaningful on 32-bit.
        if cfg!(target_pointer_width = "32") {
            let val = RedisValue::Integer((usize::MAX as i64) + 1);
            let result: Result<usize, _> = FromRedisValue::from_redis_value(&val);
            assert!(result.is_err());
        }
    }

    // --- Story 6: i32 boundary tests ---

    #[test]
    fn test_from_redis_value_i32_overflow_positive() {
        // i32::MAX + 1 is the first value outside the positive range.
        let val = RedisValue::Integer(i64::from(i32::MAX) + 1);
        let result: Result<i32, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i32_overflow_negative() {
        // i32::MIN - 1 is the first value outside the negative range.
        let val = RedisValue::Integer(i64::from(i32::MIN) - 1);
        let result: Result<i32, _> = FromRedisValue::from_redis_value(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_value_i32_near_max() {
        // i32::MAX - 1 is just inside the valid range.
        let val = RedisValue::Integer(i64::from(i32::MAX) - 1);
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i32::MAX - 1);
    }

    #[test]
    fn test_from_redis_value_i32_near_min() {
        // i32::MIN + 1 is just inside the valid range.
        let val = RedisValue::Integer(i64::from(i32::MIN) + 1);
        let result: i32 = FromRedisValue::from_redis_value(&val).unwrap();
        assert_eq!(result, i32::MIN + 1);
    }
}
