// Roundtrip tests: encode with RESPWriter, decode with RESPReader, verify equality.

use crate::base::RedisValue;

use crate::reader::RESPReader;
use crate::writer::RESPWriter;

/// Encode a value with [`RESPWriter`], then decode it with [`RESPReader`].
fn roundtrip(v: &RedisValue) -> RedisValue {
    let mut w = RESPWriter::new();
    w.write_value(v);
    let buf = w.take();
    let mut r = RESPReader::new(buf);
    r.read_value().unwrap()
}

#[test]
fn roundtrip_simple_string() {
    let expected = RedisValue::SimpleString("OK".into());
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_bulk_string() {
    let expected = RedisValue::BulkString(b"hello".to_vec());
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_integer() {
    let expected = RedisValue::Integer(42);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_integer_negative() {
    let expected = RedisValue::Integer(-1);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_null() {
    let expected = RedisValue::Null;
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_empty_array() {
    let expected = RedisValue::Array(vec![]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_nested_array() {
    let inner = RedisValue::Array(vec![
        RedisValue::BulkString(b"a".to_vec()),
        RedisValue::BulkString(b"b".to_vec()),
    ]);
    let expected = RedisValue::Array(vec![inner]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_set_command() {
    let expected = RedisValue::Array(vec![
        RedisValue::BulkString(b"SET".to_vec()),
        RedisValue::BulkString(b"key".to_vec()),
        RedisValue::BulkString(b"value".to_vec()),
        RedisValue::BulkString(b"EX".to_vec()),
        RedisValue::BulkString(b"60".to_vec()),
    ]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_keys_response() {
    let expected = RedisValue::Array(vec![
        RedisValue::BulkString(b"user:1".to_vec()),
        RedisValue::BulkString(b"user:2".to_vec()),
    ]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_large_bulk_string() {
    let payload = vec![b'x'; 65536]; // 64 KB
    let expected = RedisValue::BulkString(payload);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_error() {
    let expected = RedisValue::Error("ERR operation timed out".into());
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_deeply_nested() {
    let leaf = RedisValue::BulkString(b"dee".to_vec());
    let l4 = RedisValue::Array(vec![leaf]);
    let l3 = RedisValue::Array(vec![l4]);
    let l2 = RedisValue::Array(vec![l3]);
    let expected = RedisValue::Array(vec![l2]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}

#[test]
fn roundtrip_multi_values_in_array() {
    let expected = RedisValue::Array(vec![
        RedisValue::SimpleString("OK".into()),
        RedisValue::Integer(0),
        RedisValue::Null,
        RedisValue::BulkString(b"hello".to_vec()),
        RedisValue::Error("ERR bad".into()),
    ]);
    let actual = roundtrip(&expected);
    assert_eq!(actual, expected);
}
