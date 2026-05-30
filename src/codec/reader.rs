// RESPReader — Decode RESP2 wire format into RedisValue.
//
// Uses an internal cursor (`pos`) to track progress through the buffer.

use std::cell::Cell;

use crate::core::{RedisError, RedisValue};
use bytes::BytesMut;

/// Default maximum bulk string length (256 MB).
const DEFAULT_MAX_BULK_LEN: usize = 268_435_456;

/// Default maximum array length (1 million elements).
const DEFAULT_MAX_ARRAY_LEN: usize = 1_000_000;

/// Default maximum array nesting depth (256 levels).
pub(super) const DEFAULT_MAX_DEPTH: usize = 256;

/// RAII guard that decrements `RESPReader.depth` on drop.
///
/// FR-036, AC-4.5, AC-4.6: Prevents depth counter corruption if
/// `read_value()` panics between `depth += 1` and `depth -= 1`
/// (e.g. from OOM in `Vec::with_capacity`). If the scope exits
/// abnormally via panic or `?`, Drop still restores depth to its
/// previous value.
///
/// Attack vector: An attacker sends a deeply nested RESP array that
/// triggers OOM. Without this guard the depth counter is left
/// incremented. Subsequent reads of legitimate responses (at normal
/// depth) are incorrectly rejected as "too deep", causing a
/// denial-of-service.
pub(super) struct DepthGuard {
    // Raw pointer avoids borrow conflicts: holding a reference to RESPReader
    // prevents calling self.read_line() etc. inside read_array.
    depth_cell: *const Cell<usize>,
    saved: usize,
}

impl DepthGuard {
    #[must_use = "must hold the guard for the duration of the recursion"]
    pub(super) fn new(reader: &RESPReader) -> Self {
        let saved = reader.depth.get();
        reader.depth.set(saved + 1);
        Self {
            depth_cell: &raw const reader.depth,
            saved,
        }
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        // AC-4.5: Restore depth even if the scope exits via panic.
        // Using raw pointer dereference avoids borrow checker conflicts:
        // the guard holds no Rust reference to RESPReader, so read_array
        // can still call self.read_line(), self.expect_crlf(), etc.
        debug_assert!(
            !self.depth_cell.is_null(),
            "DepthGuard raw pointer must never be null"
        );
        // Direct raw pointer dereference avoids Option::expect/unwrap which
        // conflict with deny-level clippy lints. The debug_assert above
        // guarantees the pointer is valid (we set it from a reference in new()).
        unsafe { (*self.depth_cell).set(self.saved) };
    }
}

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
    depth: Cell<usize>,
}

impl RESPReader {
    /// Create a new `RESPReader` backed by the given buffer.
    #[must_use = "must call read_value() to decode data"]
    pub const fn new(buf: BytesMut) -> Self {
        Self {
            buf,
            pos: 0,
            max_bulk_len: DEFAULT_MAX_BULK_LEN,
            max_array_len: DEFAULT_MAX_ARRAY_LEN,
            max_depth: DEFAULT_MAX_DEPTH,
            depth: Cell::new(0),
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
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] on malformed wire format:
    /// missing CRLF, unknown RESP marker, incomplete bulk string,
    /// invalid integer length, or array length exceeding [`RESPReader`]
    /// configured limits.
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
        // AC-4.5, AC-4.8, FR-037, FR-038: Create RAII guard BEFORE checking depth.
        // DepthGuard increments depth in `new()` and decrements in `Drop`.
        // This ensures depth is restored on panic or early return.
        // Depth check happens after increment so the max_depth value represents
        // the actual recursion depth reached (AC-4.8: check before recursion).
        let _guard = DepthGuard::new(self);

        if self.depth.get() > self.max_depth {
            return Err(RedisError::Parse(format!(
                "array nesting depth exceeds maximum of {}",
                self.max_depth
            )));
        }

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
            return Err(RedisError::Parse(format!(
                "array length {} exceeds maximum of {}",
                len, self.max_array_len
            )));
        }

        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(self.read_value()?);
        }
        Ok(RedisValue::Array(items))
    }
}
