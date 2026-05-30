// RESPReader unit tests — decode tests for wire format handling.
//
// Tests basic value reading, CRLF enforcement (S15),
// length caps (S16), and array depth limits (S10).

use crate::codec::reader::RESPReader;
use crate::codec::reader::DEFAULT_MAX_DEPTH;
use crate::core::RedisError;
use crate::core::RedisValue;
use bytes::BytesMut;

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_simple_ok() {
        let buf = BytesMut::from(b"+OK\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(val, RedisValue::SimpleString(ref s) if s == "OK"));
    }

    #[test]
    fn test_read_error_msg() {
        let buf = BytesMut::from(b"-ERR unknown command\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::Error(ref s) if s == "ERR unknown command"
        ));
    }

    #[test]
    fn test_read_integer_42() {
        let buf = BytesMut::from(b":42\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert_eq!(val.as_integer(), Some(42));
    }

    #[test]
    fn test_read_bulk_string() {
        let buf = BytesMut::from(b"$5\r\nhello\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(val, RedisValue::BulkString(ref b) if b == b"hello"));
    }

    #[test]
    fn test_read_null_bulk() {
        let buf = BytesMut::from(b"$-1\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(val, RedisValue::Null));
    }

    #[test]
    fn test_read_empty_bulk_string() {
        let buf = BytesMut::from(b"$0\r\n\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::BulkString(ref b) if b.is_empty()
        ));
    }

    #[test]
    fn test_read_array_two_strings() {
        let buf = BytesMut::from(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(items) = val {
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0], RedisValue::BulkString(ref b) if b == b"foo"));
            assert!(matches!(items[1], RedisValue::BulkString(ref b) if b == b"bar"));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_read_empty_array() {
        let buf = BytesMut::from(b"*0\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(items) = val {
            assert!(items.is_empty());
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_read_nested_array() {
        let buf = BytesMut::from(b"*1\r\n*2\r\n$1\r\na\r\n$1\r\nb\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(inner) = val {
            assert_eq!(inner.len(), 1);
            if let RedisValue::Array(nested) = &inner[0] {
                assert_eq!(nested.len(), 2);
            } else {
                panic!("expected nested Array");
            }
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_read_multiple_values() {
        let buf = BytesMut::from(b"+OK\r\n:42\r\n$-1\r\n".as_ref());
        let mut r = RESPReader::new(buf);

        let v1 = r.read_value().unwrap();
        assert!(matches!(v1, RedisValue::SimpleString(ref s) if s == "OK"));

        let v2 = r.read_value().unwrap();
        assert_eq!(v2.as_integer(), Some(42));

        let v3 = r.read_value().unwrap();
        assert!(matches!(v3, RedisValue::Null));
    }

    #[test]
    fn test_read_missing_crlf_error() {
        let buf = BytesMut::from(b"+hello".as_ref());
        let mut r = RESPReader::new(buf);
        assert!(r.read_value().is_err());
    }

    #[test]
    fn test_read_invalid_marker() {
        let buf = BytesMut::from(b"X\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        assert!(r.read_value().is_err());
    }

    // -----------------------------------------------------------------------
    // S15 — Strict CRLF enforcement after every value
    // -----------------------------------------------------------------------

    #[test]
    fn test_crlf_missing_after_simple() {
        // CR without LF after value — bare \r not followed by \n
        let buf = BytesMut::from(b"+OK\rPONG\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_missing_after_int() {
        // CR without LF after integer value
        let buf = BytesMut::from(b":42\rPONG\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_missing_after_bulk() {
        // CR without LF after bulk string content
        let buf = BytesMut::from(b"$5\r\nhello\rPONG\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_missing_after_bulk_no_crlf_at_all() {
        // No CRLF whatsoever after bulk string content
        let buf = BytesMut::from(b"$5\r\nhelloPONG\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_double_lf() {
        // LF before CR — invalid line terminator
        let buf = BytesMut::from(b"+OK\n\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_null_bulk() {
        // $-1 null bulk string must still have CRLF terminator
        let buf = BytesMut::from(b"$-1\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(val, RedisValue::Null));
    }

    #[test]
    fn test_crlf_null_bulk_missing_crlf() {
        // $-1 without CRLF is invalid
        let buf = BytesMut::from(b"$-1\rX\r\n".as_ref());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match &err {
            RedisError::Parse(msg) => assert!(msg.contains("expected CRLF")),
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_crlf_empty_buffer() {
        let buf = BytesMut::new();
        let mut r = RESPReader::new(buf);
        assert!(r.read_value().is_err());
    }

    // -----------------------------------------------------------------------
    // S16 — Bulk string and array length caps
    // -----------------------------------------------------------------------

    #[test]
    fn test_bulk_under_default_cap() {
        // $1000 is well under 256 MB default
        let data: Vec<u8> = vec![b'a'; 1000];
        let payload = format!("${}\r\n{}\r\n", data.len(), String::from_utf8_lossy(&data));
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::BulkString(ref b) if b.len() == 1000
        ));
    }

    #[test]
    fn test_bulk_over_default_cap() {
        // $268435457 exceeds 256 MB default
        let payload = "$268435457\r\nx\r\n";
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 268435456"));
                assert!(msg.contains("268435457"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_bulk_custom_cap() {
        let payload = "$101\r\nxxxxxxxxxxxx\r\n";
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf).with_max_bulk_len(100);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 100"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_bulk_null_not_capped() {
        let buf = BytesMut::from(b"$-1\r\n".as_ref());
        let mut r = RESPReader::new(buf).with_max_bulk_len(0);
        let val = r.read_value().unwrap();
        assert!(matches!(val, RedisValue::Null));
    }

    #[test]
    fn test_bulk_zero_ok() {
        let buf = BytesMut::from(b"$0\r\n\r\n".as_ref());
        let mut r = RESPReader::new(buf).with_max_bulk_len(0);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::BulkString(ref b) if b.is_empty()
        ));
    }

    #[test]
    fn test_bulk_exact_cap_ok() {
        let data: Vec<u8> = vec![b'a'; 100];
        let payload = format!("${}\r\n{}\r\n", data.len(), String::from_utf8_lossy(&data));
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf).with_max_bulk_len(100);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::BulkString(ref b) if b.len() == 100
        ));
    }

    #[test]
    fn test_array_under_default_cap() {
        // Build an array of 1000 strings
        let mut items = Vec::new();
        for i in 0..1000 {
            items.push(format!("$4\r\n{i:04}\r\n"));
        }
        let payload = format!("*{}\r\n{}", 1000, items.join(""));
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(items) = val {
            assert_eq!(items.len(), 1000);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_array_over_default_cap() {
        // $-1\r\n is a null bulk string (cheap to construct)
        let payload = "*1000001\r\n$-1\r\n";
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 1000000"));
                assert!(msg.contains("1000001"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_array_custom_cap() {
        let payload = "*11\r\n$3\r\nfoo\r\n$3\r\nbar\r\n$3\r\nbaz\r\n$3\r\nqux\r\n$3\r\nquux\r\n$3\r\ncorge\r\n$3\r\ngrault\r\n$3\r\ngarply\r\n$3\r\nwaldo\r\n$3\r\nfred\r\n";
        let buf = BytesMut::from(payload.as_bytes());
        let mut r = RESPReader::new(buf).with_max_array_len(10);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 10"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_array_empty_ok() {
        let buf = BytesMut::from(b"*0\r\n".as_ref());
        let mut r = RESPReader::new(buf).with_max_array_len(0);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(items) = val {
            assert!(items.is_empty());
        } else {
            panic!("expected Array");
        }
    }

    // -----------------------------------------------------------------------
    // S10 — Array depth limit
    // -----------------------------------------------------------------------

    fn build_nested_array(depth: usize) -> Vec<u8> {
        let mut result = Vec::new();
        // Open depth arrays: *1\r\n repeated
        for _ in 0..depth {
            result.extend_from_slice(b"*1\r\n");
        }
        // Leaf: a simple bulk string
        result.extend_from_slice(b"$1\r\nx\r\n");
        result
    }

    #[test]
    fn test_depth_ok_10() {
        let buf = BytesMut::from(&build_nested_array(10)[..]);
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        // Unwrap 10 levels
        let mut current = val;
        for _ in 0..10 {
            if let RedisValue::Array(inner) = current {
                assert_eq!(inner.len(), 1);
                current = inner.into_iter().next().unwrap();
            } else {
                panic!("expected Array");
            }
        }
        assert!(matches!(current, RedisValue::BulkString(ref b) if b == b"x"));
    }

    #[test]
    fn test_depth_flat_arrays_ok() {
        // Sibling arrays should not increment depth beyond 1
        let buf = BytesMut::from(
            b"*2\r\n*3\r\n$1\r\na\r\n$1\r\nb\r\n$1\r\nc\r\n*2\r\n$1\r\nd\r\n$1\r\ne\r\n".as_ref(),
        );
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        if let RedisValue::Array(items) = val {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_depth_exactly_default_limit() {
        let buf = BytesMut::from(&build_nested_array(DEFAULT_MAX_DEPTH)[..]);
        let mut r = RESPReader::new(buf);
        let val = r.read_value().unwrap();
        // Should succeed — at the limit, not over
        let mut current = val;
        for _ in 0..DEFAULT_MAX_DEPTH {
            if let RedisValue::Array(inner) = current {
                assert_eq!(inner.len(), 1);
                current = inner.into_iter().next().unwrap();
            } else {
                panic!("expected Array");
            }
        }
        assert!(matches!(current, RedisValue::BulkString(ref b) if b == b"x"));
    }

    #[test]
    fn test_depth_exceeds_default_limit() {
        let buf = BytesMut::from(&build_nested_array(DEFAULT_MAX_DEPTH + 1)[..]);
        let mut r = RESPReader::new(buf);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 256"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_depth_custom_50() {
        let buf = BytesMut::from(&build_nested_array(51)[..]);
        let mut r = RESPReader::new(buf).with_max_depth(50);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 50"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_depth_zero_rejects_arrays() {
        let buf = BytesMut::from(b"*1\r\n$3\r\nfoo\r\n".as_ref());
        let mut r = RESPReader::new(buf).with_max_depth(0);
        let err = r.read_value().unwrap_err();
        match err {
            RedisError::Parse(msg) => {
                assert!(msg.contains("exceeds maximum of 0"));
            }
            _ => panic!("expected Parse error, got {err:?}"),
        }
    }

    #[test]
    fn test_depth_zero_allows_non_arrays() {
        let buf = BytesMut::from(b"$5\r\nhello\r\n".as_ref());
        let mut r = RESPReader::new(buf).with_max_depth(0);
        let val = r.read_value().unwrap();
        assert!(matches!(
            val,
            RedisValue::BulkString(ref b) if b == b"hello"
        ));
    }
}
