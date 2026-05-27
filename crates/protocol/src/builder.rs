// CommandBuilder — Fluent API for building Redis commands.

use base::{RedisValue, ToRedisArgs};
use bytes::BytesMut;
use codec::writer::RESPWriter;

/// A fluent builder for Redis commands.
///
/// Collects command name and arguments, then encodes them into RESP2 wire
/// format when [`build()`](Self::build) is called.
pub struct CommandBuilder {
    args: Vec<RedisValue>,
}

impl CommandBuilder {
    /// Create a new `CommandBuilder` with the given command name.
    ///
    /// The command name is converted to a `BulkString` RedisValue.
    pub fn new(cmd: &str) -> Self {
        Self {
            args: vec![RedisValue::BulkString(cmd.as_bytes().to_vec())],
        }
    }

    /// Append a single argument.
    ///
    /// The value is converted to a `RedisValue` via [`ToRedisArgs`].
    pub fn arg<V: ToRedisArgs>(mut self, val: V) -> Self {
        let mut buf = Vec::new();
        val.write_redis_args(&mut buf);
        if let Some(first) = buf.into_iter().next() {
            self.args.push(RedisValue::BulkString(first));
        }
        self
    }

    /// Append multiple arguments at once.
    pub fn args<V: ToRedisArgs>(mut self, vals: &[V]) -> Self {
        let mut buf = Vec::new();
        for item in vals {
            item.write_redis_args(&mut buf);
        }
        for item in buf {
            self.args.push(RedisValue::BulkString(item));
        }
        self
    }

    /// Encode the command into RESP2 wire format.
    pub fn build(self) -> BytesMut {
        let mut writer = RESPWriter::new();
        if self.args.len() == 1 {
            writer.write_value(&self.args[0]);
        } else {
            writer.write_array_header(self.args.len());
            for arg in &self.args {
                writer.write_value(arg);
            }
        }
        writer.take()
    }

    /// Returns the number of arguments in this command (including the command
    /// name itself).
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Returns `true` if this command has no arguments (only the command name).
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

/// Convenience function to create a `CommandBuilder`.
///
/// ```ignore
/// use protocol::cmd;
///
/// let builder = cmd("SET").arg("key").arg("value");
/// ```
pub fn cmd(cmd: &str) -> CommandBuilder {
    CommandBuilder::new(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_set_key_value() {
        let buf = cmd("SET").arg("k").arg("v").build();
        assert_eq!(buf.as_ref(), b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n");
    }

    #[test]
    fn test_cmd_get_key() {
        let buf = cmd("GET").arg("key").build();
        assert_eq!(buf.as_ref(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_cmd_with_multiple_args() {
        let buf = cmd("MSET")
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
        assert_eq!(cmd("SET").arg("k").arg("v").len(), 3);
    }

    #[test]
    fn test_cmd_is_empty() {
        assert!(cmd("PING").is_empty());
        assert!(!cmd("SET").arg("k").is_empty());
    }

    #[test]
    fn test_cmd_with_int_arg() {
        let buf = cmd("INCR").arg(42i64).build();
        assert_eq!(buf.as_ref(), b"*2\r\n$4\r\nINCR\r\n$2\r\n42\r\n");
    }
}
