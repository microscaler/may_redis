// Roundtrip invariant tests for all RedisValue variants
//
// These tests verify that write_value → read_value is an identity function
// for every possible RedisValue input. Each test follows the pattern:
//   1. Write the value using RESPWriter
//   2. Read it back using RESPReader
//   3. Assert equality with the original

use crate::core::RedisValue;
use crate::codec::reader::RESPReader;
use crate::codec::writer::RESPWriter;

/// Helper: encode a value then decode it, asserting equality.
fn roundtrip(value: &RedisValue) -> RedisValue {
    let mut w = RESPWriter::new();
    w.write_value(value);
    let bytes = w.take();
    let mut r = RESPReader::new(bytes);
    r.read_value()
        .expect("write_value/read_value roundtrip failed")
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod simple_string {
    use super::*;

    #[test]
    fn test_roundtrip_simple_ok() {
        let input = RedisValue::SimpleString("OK".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s == "OK"));
    }

    #[test]
    fn test_roundtrip_simple_with_spaces() {
        let input = RedisValue::SimpleString("OK with spaces".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s == "OK with spaces"));
    }

    #[test]
    fn test_roundtrip_simple_long() {
        let input = RedisValue::SimpleString("A".repeat(1000));
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::SimpleString(ref s) if s.len() == 1000 && *s == "A".repeat(1000))
        );
    }

    #[test]
    fn test_roundtrip_simple_empty() {
        let input = RedisValue::SimpleString(String::new());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s.is_empty()));
    }

    #[test]
    fn test_roundtrip_simple_unicode() {
        let input = RedisValue::SimpleString("你好世界".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s == "你好世界"));
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod error_value {
    use super::*;

    #[test]
    fn test_roundtrip_error_no_auth() {
        let input = RedisValue::Error("-NOAUTH Authentication required.".into());
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::Error(ref s) if s == "-NOAUTH Authentication required.")
        );
    }

    #[test]
    fn test_roundtrip_error_wrong_type() {
        let input = RedisValue::Error("ERR wrong type".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Error(ref s) if s == "ERR wrong type"));
    }

    #[test]
    fn test_roundtrip_error_empty() {
        let input = RedisValue::Error(String::new());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Error(ref s) if s.is_empty()));
    }

    #[test]
    fn test_roundtrip_error_long() {
        let input = RedisValue::Error("ERR ".to_string() + &"x".repeat(500));
        let output = roundtrip(&input);
        let expected = format!("ERR {}", "x".repeat(500));
        assert!(
            matches!(output, RedisValue::Error(ref s) if *s == expected)
        );
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod integer_value {
    use super::*;

    #[test]
    fn test_roundtrip_int_zero() {
        let input = RedisValue::Integer(0);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == 0));
    }

    #[test]
    fn test_roundtrip_int_positive() {
        let input = RedisValue::Integer(42);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == 42));
    }

    #[test]
    fn test_roundtrip_int_negative() {
        let input = RedisValue::Integer(-1);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == -1));
    }

    #[test]
    fn test_roundtrip_int_max() {
        let input = RedisValue::Integer(i64::MAX);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == i64::MAX));
    }

    #[test]
    fn test_roundtrip_int_min() {
        let input = RedisValue::Integer(i64::MIN);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == i64::MIN));
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod bulk_string {
    use super::*;

    #[test]
    fn test_roundtrip_bulk_ascii() {
        let input = RedisValue::BulkString(b"hello".to_vec());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::BulkString(ref b) if b == b"hello"));
    }

    #[test]
    fn test_roundtrip_bulk_empty() {
        let input = RedisValue::BulkString(Vec::new());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::BulkString(ref b) if b.is_empty()));
    }

    #[test]
    fn test_roundtrip_binary_non_utf8() {
        let input = RedisValue::BulkString(vec![0x00, 0xFF, 0x80, 0x7F, 0x41, 0x00]);
        let expected = vec![0x00, 0xFF, 0x80, 0x7F, 0x41, 0x00];
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::BulkString(ref b) if *b == expected)
        );
    }

    #[test]
    fn test_roundtrip_bulk_all_256_bytes() {
        let data: Vec<u8> = (0u8..=255).collect();
        let input = RedisValue::BulkString(data);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::BulkString(ref b) if b.len() == 256));
    }

    #[test]
    fn test_roundtrip_bulk_large() {
        let data = vec![b'a'; 65_536];
        let input = RedisValue::BulkString(data);
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::BulkString(ref b) if b.len() == 65_536 && b[0] == b'a')
        );
    }

    #[test]
    fn test_roundtrip_bulk_unicode() {
        let input = RedisValue::BulkString("こんにちは世界".as_bytes().to_vec());
        let output = roundtrip(&input);
        let expected: &[u8] = "こんにちは世界".as_bytes();
        assert!(
            matches!(output, RedisValue::BulkString(ref b) if *b == expected)
        );
    }

    #[test]
    fn test_roundtrip_bulk_null_terminated() {
        let input = RedisValue::BulkString(vec![b'h', b'i', 0x00, b'!', 0x00]);
        let expected = vec![b'h', b'i', 0x00, b'!', 0x00];
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::BulkString(ref b) if *b == expected)
        );
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod null_value {
    use super::*;

    #[test]
    fn test_roundtrip_null() {
        let input = RedisValue::Null;
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Null));
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod array_value {
    use super::*;

    #[test]
    fn test_roundtrip_array_empty() {
        let input = RedisValue::Array(vec![]);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Array(ref a) if a.is_empty()));
    }

    #[test]
    fn test_roundtrip_array_single_int() {
        let input = RedisValue::Array(vec![RedisValue::Integer(42)]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 1);
            assert!(matches!(a[0], RedisValue::Integer(42)));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_array_mixed_types() {
        let input = RedisValue::Array(vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(b"hi".to_vec()),
            RedisValue::Null,
        ]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 3);
            assert!(matches!(a[0], RedisValue::Integer(1)));
            assert!(matches!(a[1], RedisValue::BulkString(ref b) if b == b"hi"));
            assert!(matches!(a[2], RedisValue::Null));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_nested_array() {
        let input = RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::Array(vec![RedisValue::Integer(42)])
        ])]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 1);
            if let RedisValue::Array(inner) = &a[0] {
                assert_eq!(inner.len(), 1);
                if let RedisValue::Array(nested) = &inner[0] {
                    assert_eq!(nested.len(), 1);
                    assert!(matches!(nested[0], RedisValue::Integer(42)));
                } else {
                    panic!("expected nested Array");
                }
            } else {
                panic!("expected inner Array");
            }
        } else {
            panic!("expected outer Array");
        }
    }

    #[test]
    fn test_roundtrip_array_with_error() {
        let input = RedisValue::Array(vec![
            RedisValue::Error("ERR x".into()),
            RedisValue::Integer(0),
        ]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 2);
            assert!(matches!(a[0], RedisValue::Error(ref s) if s == "ERR x"));
            assert!(matches!(a[1], RedisValue::Integer(0)));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_deep_nesting() {
        fn nested_array(depth: usize) -> RedisValue {
            if depth == 0 {
                RedisValue::Integer(42)
            } else {
                RedisValue::Array(vec![nested_array(depth - 1)])
            }
        }
        fn count_depth_helper(val: &RedisValue) -> usize {
            match val {
                RedisValue::Array(a) => 1 + a.first().map_or(0, count_depth_helper),
                RedisValue::Integer(42) => 0,
                _ => usize::MAX, // mismatch
            }
        }
        // 5 levels of nesting
        let input = nested_array(5);
        let output = roundtrip(&input);

        // Verify the depth-0 leaf is still an Integer(42)
        assert_eq!(count_depth_helper(&output), 5); // 5 Array wrappers around Integer(42)
    }

    #[test]
    fn test_roundtrip_many_elements() {
        let input = RedisValue::Array(vec![RedisValue::Integer(1); 1000]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 1000);
            assert!(a.iter().all(|v| matches!(v, RedisValue::Integer(1))));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_array_with_simple_strings() {
        let input = RedisValue::Array(vec![
            RedisValue::SimpleString("OK".into()),
            RedisValue::SimpleString("PONG".into()),
        ]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 2);
            assert!(matches!(a[0], RedisValue::SimpleString(ref s) if s == "OK"));
            assert!(matches!(a[1], RedisValue::SimpleString(ref s) if s == "PONG"));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_array_empty_nested() {
        let input = RedisValue::Array(vec![RedisValue::Array(vec![]), RedisValue::Integer(0)]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 2);
            assert!(matches!(a[0], RedisValue::Array(ref b) if b.is_empty()));
            assert!(matches!(a[1], RedisValue::Integer(0)));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_mixed_array_deeply_nested() {
        let input = RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(b"nested".to_vec()),
            RedisValue::Array(vec![RedisValue::Null]),
        ])]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 1);
            if let RedisValue::Array(inner) = &a[0] {
                assert_eq!(inner.len(), 3);
                assert!(matches!(inner[0], RedisValue::Integer(1)));
                assert!(matches!(inner[1], RedisValue::BulkString(ref b) if b == b"nested"));
                assert!(
                    matches!(inner[2], RedisValue::Array(ref c) if c.len() == 1 && matches!(c[0], RedisValue::Null))
                );
            } else {
                panic!("expected inner Array");
            }
        } else {
            panic!("expected outer Array");
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod boundary_cases {
    use super::*;

    #[test]
    fn test_roundtrip_single_char_bulk() {
        let input = RedisValue::BulkString(vec![b'!']);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::BulkString(ref b) if b == b"!"));
    }

    #[test]
    fn test_roundtrip_1_byte_simple_string() {
        let input = RedisValue::SimpleString("A".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s == "A"));
    }

    #[test]
    fn test_roundtrip_1_byte_int() {
        let input = RedisValue::Integer(1);
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::Integer(n) if n == 1));
    }

    #[test]
    fn test_roundtrip_array_of_nulls() {
        let input = RedisValue::Array(vec![RedisValue::Null; 10]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 10);
            assert!(a.iter().all(|v| matches!(v, RedisValue::Null)));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_array_of_arrays_of_arrays() {
        // Triple-nested single-element arrays
        let input = RedisValue::Array(vec![RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::BulkString(b"deep".to_vec()),
        ])])]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            if let RedisValue::Array(b) = &a[0] {
                if let RedisValue::Array(c) = &b[0] {
                    assert!(matches!(c[0], RedisValue::BulkString(ref v) if v == b"deep"));
                } else {
                    panic!("expected triple-nested Array");
                }
            } else {
                panic!("expected double-nested Array");
            }
        } else {
            panic!("expected outer Array");
        }
    }

    #[test]
    fn test_roundtrip_large_array_of_integers() {
        let input: RedisValue = RedisValue::Array((0..500).map(RedisValue::Integer).collect());
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 500);
            for (i, v) in a.iter().enumerate() {
                assert!(matches!(v, RedisValue::Integer(n) if *n == i64::try_from(i).expect("fits in i64")));
            }
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_error_with_nulls_in_array() {
        let input = RedisValue::Array(vec![
            RedisValue::Error("ERR".into()),
            RedisValue::Null,
            RedisValue::Integer(-1),
            RedisValue::BulkString(vec![]),
        ]);
        let output = roundtrip(&input);
        if let RedisValue::Array(a) = output {
            assert_eq!(a.len(), 4);
            assert!(matches!(a[0], RedisValue::Error(ref s) if s == "ERR"));
            assert!(matches!(a[1], RedisValue::Null));
            assert!(matches!(a[2], RedisValue::Integer(-1)));
            assert!(matches!(a[3], RedisValue::BulkString(ref b) if b.is_empty()));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_roundtrip_simple_string_with_comma() {
        let input = RedisValue::SimpleString("OK, good".into());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::SimpleString(ref s) if s == "OK, good"));
    }

    #[test]
    fn test_roundtrip_bulk_with_cr_and_lf_bytes() {
        // CRLF bytes should be preserved as raw data (they are inside a bulk string, not as protocol terminators)
        let input = RedisValue::BulkString(b"\r\n\r\n".to_vec());
        let output = roundtrip(&input);
        assert!(matches!(output, RedisValue::BulkString(ref b) if b == b"\r\n\r\n"));
    }

    #[test]
    fn test_roundtrip_bulk_with_zero_byte_at_end() {
        let input = RedisValue::BulkString(vec![b'a', b'b', b'c', 0x00]);
        let expected = vec![b'a', b'b', b'c', 0x00];
        let output = roundtrip(&input);
        assert!(
            matches!(output, RedisValue::BulkString(ref b) if *b == expected)
        );
    }
}
