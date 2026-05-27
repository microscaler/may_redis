// RESPWriter — Encode RedisValue into RESP2 wire format.

use base::RedisValue;
use bytes::BytesMut;

/// Writer that produces RESP2 wire format from [`RedisValue`].
///
/// All encoding happens into an internal `BytesMut` buffer. Call
/// [`take()`](Self::take) to drain the buffer and start a new empty one.
pub struct RESPWriter {
    buf: BytesMut,
}

impl RESPWriter {
    /// Create a new `RESPWriter` with a small default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Create a `RESPWriter` with the given initial capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(cap),
        }
    }

    /// Write a simple string: `+{s}\r\n`.
    pub fn write_simple(&mut self, s: &str) {
        self.buf.extend_from_slice(b"+");
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.extend_from_slice(b"\r\n");
    }

    /// Write a bulk string: `${len}\r\n{data}\r\n`.
    pub fn write_bulk(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(b"$");
        self.buf
            .extend_from_slice(itoa::Buffer::new().format(data.len()).as_bytes());
        self.buf.extend_from_slice(b"\r\n");
        self.buf.extend_from_slice(data);
        self.buf.extend_from_slice(b"\r\n");
    }

    /// Write an integer: `:{n}\r\n`.
    pub fn write_int(&mut self, n: i64) {
        self.buf.extend_from_slice(b":");
        self.buf
            .extend_from_slice(itoa::Buffer::new().format(n).as_bytes());
        self.buf.extend_from_slice(b"\r\n");
    }

    /// Write an array header: `*{len}\r\n`.
    pub fn write_array_header(&mut self, len: usize) {
        self.buf.extend_from_slice(b"*");
        self.buf
            .extend_from_slice(itoa::Buffer::new().format(len).as_bytes());
        self.buf.extend_from_slice(b"\r\n");
    }

    /// Encode a single [`RedisValue`] into the buffer by dispatching to the
    /// appropriate write method.
    pub fn write_value(&mut self, v: &RedisValue) {
        match v {
            RedisValue::SimpleString(s) => self.write_simple(s),
            RedisValue::BulkString(b) => self.write_bulk(b),
            RedisValue::Integer(n) => self.write_int(*n),
            RedisValue::Error(msg) => self.write_error(msg),
            RedisValue::Null => self.write_null_bulk(),
            RedisValue::Array(arr) => {
                self.write_array_header(arr.len());
                for elem in arr {
                    self.write_value(elem);
                }
            }
        }
    }

    /// Write a null bulk string: `$-1\r\n`.
    pub fn write_null_bulk(&mut self) {
        self.buf.extend_from_slice(b"$-1\r\n");
    }

    /// Write an empty array: `*0\r\n`.
    pub fn write_empty_array(&mut self) {
        self.buf.extend_from_slice(b"*0\r\n");
    }

    /// Write a protocol error: `-{msg}\r\n`.
    pub fn write_error(&mut self, msg: &str) {
        self.buf.extend_from_slice(b"-");
        self.buf.extend_from_slice(msg.as_bytes());
        self.buf.extend_from_slice(b"\r\n");
    }

    /// Drain the buffer, returning its contents and starting a fresh one.
    pub fn take(&mut self) -> BytesMut {
        std::mem::take(&mut self.buf)
    }
}

impl Default for RESPWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_simple_ok() {
        let mut w = RESPWriter::new();
        w.write_simple("OK");
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"+OK\r\n");
    }

    #[test]
    fn test_write_bulk_hello() {
        let mut w = RESPWriter::new();
        w.write_bulk(b"hello");
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_write_int_42() {
        let mut w = RESPWriter::new();
        w.write_int(42);
        let buf = w.take();
        assert_eq!(buf.as_ref(), b":42\r\n");
    }

    #[test]
    fn test_write_int_negative() {
        let mut w = RESPWriter::new();
        w.write_int(-1);
        let buf = w.take();
        assert_eq!(buf.as_ref(), b":-1\r\n");
    }

    #[test]
    fn test_write_array_header_3() {
        let mut w = RESPWriter::new();
        w.write_array_header(3);
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"*3\r\n");
    }

    #[test]
    fn test_write_null_bulk() {
        let mut w = RESPWriter::new();
        w.write_null_bulk();
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"$-1\r\n");
    }

    #[test]
    fn test_write_empty_array() {
        let mut w = RESPWriter::new();
        w.write_empty_array();
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"*0\r\n");
    }

    #[test]
    fn test_write_error() {
        let mut w = RESPWriter::new();
        w.write_error("ERR unknown command");
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"-ERR unknown command\r\n");
    }

    #[test]
    fn test_take_returns_and_resets() {
        let mut w = RESPWriter::new();
        w.write_simple("first");
        let buf1 = w.take();
        assert_eq!(buf1.as_ref(), b"+first\r\n");
        assert!(w.take().is_empty());
        w.write_simple("second");
        let buf2 = w.take();
        assert_eq!(buf2.as_ref(), b"+second\r\n");
    }

    #[test]
    fn test_write_value_dispatch() {
        let mut w = RESPWriter::new();
        w.write_value(&RedisValue::SimpleString("OK".into()));
        w.write_value(&RedisValue::Integer(42));
        w.write_value(&RedisValue::Null);
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"+OK\r\n:42\r\n$-1\r\n");
    }

    #[test]
    fn test_write_value_array() {
        let mut w = RESPWriter::new();
        let arr = RedisValue::Array(vec![
            RedisValue::BulkString(b"foo".into()),
            RedisValue::BulkString(b"bar".into()),
        ]);
        w.write_value(&arr);
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    #[test]
    fn test_write_empty_bulk_string() {
        let mut w = RESPWriter::new();
        w.write_bulk(b"");
        let buf = w.take();
        assert_eq!(buf.as_ref(), b"$0\r\n\r\n");
    }

    #[test]
    fn test_write_large_payload() {
        let mut w = RESPWriter::new();
        let data = vec![b'a'; 65536];
        w.write_bulk(&data);
        let buf = w.take();
        assert!(buf.as_ref().starts_with(b"$65536\r\n"));
        assert!(buf.as_ref().ends_with(b"\r\n"));
        assert_eq!(buf.len(), 65546);
    }
}
