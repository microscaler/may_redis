// RESPReader — Decode RESP2 wire format into RedisValue.
//
// Uses an internal cursor (`pos`) to track progress through the buffer.

use crate::core::{RedisError, RedisValue};
use bytes::BytesMut;

/// Default maximum bulk string length (256 MB).
const DEFAULT_MAX_BULK_LEN: usize = 268_435_456;

/// Default maximum array length (1 million elements).
const DEFAULT_MAX_ARRAY_LEN: usize = 1_000_000;

/// Default maximum array nesting depth (256 levels).
const DEFAULT_MAX_DEPTH: usize = 256;

/// Reader that decodes RESP2 wire format into [`RedisValue`].
///
/// Uses an internal cursor (`pos`) to track progress through the buffer.
/// Enforces length and depth caps to prevent OOM and stack overflow from
/// malicious or malformed responses.
///
/// Strict CRLF enforcement: after every parsed value the reader expects
/// `\r\n`. Missing or malformed CRLF (e.g. bare `\n` or `\r` without its
/// companion) returns a parse error.
pub struct RESPReader {
    buf: BytesMut,
    pos: usize,
    max_bulk_len: usize,
    max_array_len: usize,
    max_depth: usize,
    depth: usize,
}

impl RESPReader {
    /// Create a new `RESPReader` backed by the given buffer.
    #[must_use = "must call read_value() to decode data"]
    pub fn new(buf: BytesMut) -> Self {
        Self {
            buf,
            pos: 0,
            max_bulk_len: DEFAULT_MAX_BULK_LEN,
            max_array_len: DEFAULT_MAX_ARRAY_LEN,
            max_depth: DEFAULT_MAX_DEPTH,
            depth: 0,
        }
    }

    /// Set maximum bulk string length (default 256 MB).
    #[must_use]
    pub const fn with_max_bulk_len(mut self, cap: usize) -> Self {
        self.max_bulk_len = cap;
        self
    }

    /// Set maximum array length (default 1 million elements).
    #[must_use]
    pub const fn with_max_array_len(mut self, cap: usize) -> Self {
        self.max_array_len = cap;
        self
    }

    /// Set maximum array nesting depth (default 256 levels).
    #[must_use]
    pub const fn with_max_depth(mut self, cap: usize) -> Self {
        self.max_depth = cap;
        self
    }

    /// Take ownership of the unconsumed portion of the buffer.
    ///
    /// Returns the bytes from `pos` to end. Used by the connection loop
    /// to recover data when decode fails partway through a value.
    #[must_use]
    pub fn take_buf(self) -> BytesMut {
        let mut buf = self.buf;
        buf.split_off(self.pos)
    }

    /// Read a single RESP value from the buffer.
    pub fn read_value(&mut self) -> Result<RedisValue, RedisError> {
        self.skip_crlf();
        let marker = self.next_byte()?;
        match marker {
            b'+' => self.read_simple(),
            b'-' => self.read_error(),
            b':' => self.read_integer(),
            b'$' => self.read_bulk(),
            b'*' => self.read_array(),
            other => Err(RedisError::Parse(format!("unknown RESP marker: {other}"))),
        }
    }

    fn next_byte(&mut self) -> Result<u8, RedisError> {
        if self.pos >= self.buf.len() {
            return Err(RedisError::Parse(
                "unexpected end of buffer in next_byte".into(),
            ));
        }
        let byte = self.buf[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    /// Silently skip optional leading CRLF (for inter-value CRLF between
    /// previously-read values). This is the pre-existing behaviour.
    fn skip_crlf(&mut self) {
        if self.pos + 2 <= self.buf.len()
            && self.buf[self.pos] == b'\r'
            && self.buf[self.pos + 1] == b'\n'
        {
            self.pos += 2;
        }
    }

    /// Enforce mandatory CRLF after a value. Returns an error if the buffer
    /// still has data but `\r\n` is not present at the current position.
    /// If the buffer is exhausted, returns `Ok(())` (nothing more to read).
    fn expect_crlf(&mut self) -> Result<(), RedisError> {
        if self.pos >= self.buf.len() {
            return Ok(());
        }
        if self.pos + 2 <= self.buf.len()
            && self.buf[self.pos] == b'\r'
            && self.buf[self.pos + 1] == b'\n'
        {
            self.pos += 2;
            Ok(())
        } else {
            Err(RedisError::Parse("expected CRLF after value".into()))
        }
    }

    /// Read a line up to `\r` (does NOT consume `\r\n`).
    ///
    /// Returns the line content as an owned `Vec<u8>`. If the buffer ends
    /// without finding `\r` or encounters a bare `\n` (not preceded by `\r`),
    /// returns a parse error.
    fn read_line(&mut self) -> Result<Vec<u8>, RedisError> {
        let start = self.pos;
        while self.pos < self.buf.len() {
            if self.buf[self.pos] == b'\r' {
                // Found `\r` — return data before it without consuming `\r\n` yet.
                // The caller must invoke `expect_crlf()` to validate and consume `\r\n`.
                let line = self.buf[start..self.pos].to_vec();
                return Ok(line);
            }
            if self.buf[self.pos] == b'\n' {
                return Err(RedisError::Parse("expected CRLF after value".into()));
            }
            self.pos += 1;
        }
        Err(RedisError::Parse(
            "unexpected end of buffer in read_line".into(),
        ))
    }

    /// Read exactly `n` bytes of content (does NOT consume trailing `\r\n`).
    ///
    /// Returns the content as an owned `Vec<u8>`. The caller must invoke
    /// `expect_crlf()` after reading content to validate the trailing `\r\n`.
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, RedisError> {
        if self.pos + n > self.buf.len() {
            return Err(RedisError::Parse(format!(
                "expected {n} bytes but only {} available",
                self.buf.len() - self.pos
            )));
        }
        let data = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(data)
    }

    fn read_simple(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        self.expect_crlf()?;
        Ok(RedisValue::SimpleString(
            String::from_utf8_lossy(&line).into_owned(),
        ))
    }

    fn read_error(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        self.expect_crlf()?;
        Ok(RedisValue::Error(
            String::from_utf8_lossy(&line).into_owned(),
        ))
    }

    fn read_integer(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        self.expect_crlf()?;
        let n = std::str::from_utf8(&line)
            .map_err(|_| RedisError::Parse("integer line is not valid UTF-8".into()))?
            .parse::<i64>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid integer: {}",
                    String::from_utf8_lossy(&line)
                ))
            })?;
        Ok(RedisValue::Integer(n))
    }

    fn read_bulk(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        self.expect_crlf()?;
        let len = std::str::from_utf8(&line)
            .map_err(|_| RedisError::Parse("bulk string length is not valid UTF-8".into()))?
            .parse::<isize>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid bulk string length: {}",
                    String::from_utf8_lossy(&line)
                ))
            })?;

        match len.cmp(&0) {
            std::cmp::Ordering::Less => {
                // $-1 (null bulk string) — CRLF already consumed by expect_crlf above
                Ok(RedisValue::Null)
            }
            std::cmp::Ordering::Equal => {
                let _ = self.read_bytes(0)?;
                self.expect_crlf()?;
                Ok(RedisValue::BulkString(Vec::new()))
            }
            std::cmp::Ordering::Greater => {
                let len = len as usize;
                if len > self.max_bulk_len {
                    return Err(RedisError::Parse(format!(
                        "bulk string length {} exceeds maximum of {}",
                        len, self.max_bulk_len
                    )));
                }
                let data = self.read_bytes(len)?;
                self.expect_crlf()?;
                Ok(RedisValue::BulkString(data))
            }
        }
    }

    fn read_array(&mut self) -> Result<RedisValue, RedisError> {
        // Enforce depth limit before recursing.
        if self.depth >= self.max_depth {
            return Err(RedisError::Parse(format!(
                "array nesting depth exceeds maximum of {}",
                self.max_depth
            )));
        }
        self.depth += 1;

        let line = self.read_line()?;
        self.expect_crlf()?;
        let len = std::str::from_utf8(&line)
            .map_err(|_| RedisError::Parse("array length is not valid UTF-8".into()))?
            .parse::<usize>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid array length: {}",
                    String::from_utf8_lossy(&line)
                ))
            })?;

        if len > self.max_array_len {
            self.depth -= 1;
            return Err(RedisError::Parse(format!(
                "array length {} exceeds maximum of {}",
                len, self.max_array_len
            )));
        }

        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(self.read_value()?);
        }
        self.depth -= 1;
        Ok(RedisValue::Array(items))
    }
}

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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            items.push(format!("${}\r\n{}\r\n", 4, format!("{:04}", i)));
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
            _ => panic!("expected Parse error, got {:?}", err),
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
