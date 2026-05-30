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
