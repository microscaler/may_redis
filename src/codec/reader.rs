// RESPReader — Decode RESP2 wire format into RedisValue.
//
// Uses an internal cursor (`pos`) to track progress through the buffer.

use crate::base::{RedisError, RedisValue};
use bytes::BytesMut;

/// Reader that decodes RESP2 wire format into [`RedisValue`].
///
/// Uses an internal cursor (`pos`) to track progress through the buffer.
pub struct RESPReader {
    buf: BytesMut,
    pos: usize,
}

impl RESPReader {
    /// Create a new `RESPReader` backed by the given buffer.
    #[must_use = "must call read_value() to decode data"]
    pub const fn new(buf: BytesMut) -> Self {
        Self { buf, pos: 0 }
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

    fn skip_crlf(&mut self) {
        if self.pos + 2 <= self.buf.len()
            && self.buf[self.pos] == b'\r'
            && self.buf[self.pos + 1] == b'\n'
        {
            self.pos += 2;
        }
    }

    fn read_line(&mut self) -> Result<&[u8], RedisError> {
        let start = self.pos;
        while self.pos < self.buf.len() {
            if self.buf[self.pos] == b'\r'
                && self.pos + 1 < self.buf.len()
                && self.buf[self.pos + 1] == b'\n'
            {
                let line = &self.buf[start..self.pos];
                self.pos += 2;
                return Ok(line);
            }
            self.pos += 1;
        }
        Err(RedisError::Parse(
            "unexpected end of buffer in read_line".into(),
        ))
    }

    fn read_bytes(&mut self, n: usize) -> Result<&[u8], RedisError> {
        if self.pos + n > self.buf.len() {
            return Err(RedisError::Parse(format!(
                "expected {n} bytes but only {} available",
                self.buf.len() - self.pos
            )));
        }
        let data = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        if self.pos + 2 <= self.buf.len()
            && self.buf[self.pos] == b'\r'
            && self.buf[self.pos + 1] == b'\n'
        {
            self.pos += 2;
        }
        Ok(data)
    }

    fn read_simple(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        Ok(RedisValue::SimpleString(
            String::from_utf8_lossy(line).into_owned(),
        ))
    }

    fn read_error(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        Ok(RedisValue::Error(
            String::from_utf8_lossy(line).into_owned(),
        ))
    }

    fn read_integer(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        let n = std::str::from_utf8(line)
            .map_err(|_| RedisError::Parse("integer line is not valid UTF-8".into()))?
            .parse::<i64>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid integer: {}",
                    String::from_utf8_lossy(line)
                ))
            })?;
        Ok(RedisValue::Integer(n))
    }

    fn read_bulk(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        let len = std::str::from_utf8(line)
            .map_err(|_| RedisError::Parse("bulk string length is not valid UTF-8".into()))?
            .parse::<isize>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid bulk string length: {}",
                    String::from_utf8_lossy(line)
                ))
            })?;

        match len.cmp(&0) {
            std::cmp::Ordering::Less => Ok(RedisValue::Null),
            std::cmp::Ordering::Equal => {
                let _ = self.read_bytes(0)?;
                Ok(RedisValue::BulkString(Vec::new()))
            }
            std::cmp::Ordering::Greater => {
                let data = self.read_bytes(len as usize)?;
                Ok(RedisValue::BulkString(data.to_vec()))
            }
        }
    }

    fn read_array(&mut self) -> Result<RedisValue, RedisError> {
        let line = self.read_line()?;
        let len = std::str::from_utf8(line)
            .map_err(|_| RedisError::Parse("array length is not valid UTF-8".into()))?
            .parse::<usize>()
            .map_err(|_| {
                RedisError::Parse(format!(
                    "invalid array length: {}",
                    String::from_utf8_lossy(line)
                ))
            })?;

        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(self.read_value()?);
        }
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
        let buf = BytesMut::from(b"*1\r\n*2\r\n$3\r\na\r\n$3\r\nb\r\n".as_ref());
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
}
