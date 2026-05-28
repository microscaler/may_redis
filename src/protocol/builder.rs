// CommandBuilder — Fluent API for building Redis commands.

use crate::codec::writer::RESPWriter;
use crate::core::{RedisValue, ToRedisArgs};
use bytes::BytesMut;

/// A fluent builder for Redis commands.
///
/// Collects command name and arguments, then encodes them into RESP2 wire
/// format when [`build()`](Self::build) is called.
#[derive(Clone)]
pub struct CommandBuilder {
    args: Vec<RedisValue>,
    buf: Vec<Vec<u8>>,
}

impl CommandBuilder {
    /// Create a new `CommandBuilder` with the given command name.
    ///
    /// The command name is converted to a `BulkString` `RedisValue`.
    #[must_use]
    pub fn new(cmd: &str) -> Self {
        Self {
            args: vec![RedisValue::BulkString(cmd.as_bytes().to_vec())],
            buf: Vec::new(),
        }
    }

    /// Append a single argument.
    ///
    /// The value is converted to a `RedisValue` via [`ToRedisArgs`].
    #[allow(clippy::needless_pass_by_value)]
    #[must_use = "returns a new CommandBuilder"]
    pub fn arg<V: ToRedisArgs>(mut self, val: V) -> Self {
        self.buf.clear();
        val.write_redis_args(&mut self.buf);
        for item in self.buf.drain(..) {
            self.args.push(RedisValue::BulkString(item));
        }
        self
    }

    /// Append multiple arguments at once.
    #[must_use = "returns a new CommandBuilder"]
    pub fn args<V: ToRedisArgs>(mut self, vals: &[V]) -> Self {
        self.buf.clear();
        for item in vals {
            item.write_redis_args(&mut self.buf);
        }
        for item in self.buf.drain(..) {
            self.args.push(RedisValue::BulkString(item));
        }
        self
    }

    /// Encode the command into RESP2 wire format.
    #[must_use]
    pub fn build(self) -> BytesMut {
        let mut writer = RESPWriter::new();
        writer.write_array_header(self.args.len());
        for arg in &self.args {
            writer.write_value(arg);
        }
        writer.take()
    }

    /// Returns the number of arguments in this command (including the command
    /// name itself).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.args.len()
    }

    /// Returns `true` if this command has only the command name (no additional
    /// arguments).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.args.len() <= 1
    }
}

/// Convenience function to create a `CommandBuilder`.
///
/// ```no_run
/// use may_redis::cmd;
///
/// let builder = cmd("SET").arg("key").arg("value");
/// ```
#[must_use]
pub fn cmd(cmd: &str) -> CommandBuilder {
    CommandBuilder::new(cmd)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_set_key_value() {
        let builder = cmd("SET");
        let buf = builder.arg("k").arg("v").build();
        assert_eq!(buf.as_ref(), b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n");
    }

    #[test]
    fn test_cmd_get_key() {
        let builder = cmd("GET");
        let buf = builder.arg("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_cmd_with_multiple_args() {
        let builder = cmd("MSET");
        let buf = builder
            .args(&[
                "k1".to_string(),
                "v1".to_string(),
                "k2".to_string(),
                "v2".to_string(),
            ])
            .build();
        assert_eq!(
            buf.as_ref(),
            b"*5\r\n$4\r\nMSET\r\n$2\r\nk1\r\n$2\r\nv1\r\n$2\r\nk2\r\n$2\r\nv2\r\n"
        );
    }

    #[test]
    fn test_cmd_len() {
        assert_eq!(cmd("PING").len(), 1);
    }

    #[test]
    fn test_cmd_len_with_args() {
        let builder = cmd("SET");
        assert_eq!(builder.arg("k").arg("v").len(), 3);
    }

    #[test]
    fn test_cmd_is_empty() {
        assert!(cmd("PING").is_empty());
        assert!(!cmd("SET").arg("k").is_empty());
    }

    #[test]
    fn test_cmd_with_int_arg() {
        let builder = cmd("INCR");
        let buf = builder.arg(42i64).build();
        assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINCR\r\n$2\r\n42\r\n");
    }
}
