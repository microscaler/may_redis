// CommandBuilder — Fluent API for building Redis commands.

use crate::codec::writer::RESPWriter;
use crate::core::{RedisValue, ToRedisArgs};
use bytes::BytesMut;

/// Policy for controlling which Redis commands are allowed.
///
/// Used by the `CommandBuilder` to validate commands before building them
/// into RESP format. Prevents potentially dangerous commands from being
/// executed (e.g. `FLUSHALL`, `KEYS`, `DEBUG`).
///
/// AC-3.11: Commands are validated at build time, not at execution time.
/// If a command is blocked, [`build`] returns `None` and no data is
/// sent to the connection loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandPolicy {
    /// Allow dangerous write commands (FLUSHALL, FLUSHDB, DEBUG, SHUTDOWN).
    /// When `false` (default), these commands are blocked.
    pub allow_dangerous_writes: bool,
    /// Allow scan-heavy commands (KEYS, RANDOMKEY).
    /// When `false` (default), these commands are blocked.
    pub allow_scan_heavy: bool,
}

impl CommandPolicy {
    /// Default policy: block all dangerous and scan-heavy commands.
    pub const DEFAULT: Self = Self {
        allow_dangerous_writes: false,
        allow_scan_heavy: false,
    };

    /// Allow all commands (no restrictions).
    pub const PERMISSIVE: Self = Self {
        allow_dangerous_writes: true,
        allow_scan_heavy: true,
    };

    /// Check if a command is allowed by this policy.
    pub fn is_allowed(&self, cmd: &str) -> bool {
        let cmd_upper = cmd.to_ascii_uppercase();

        // Dangerous write commands
        if !self.allow_dangerous_writes {
            match cmd_upper.as_str() {
                "FLUSHALL" | "FLUSHDB" | "DEBUG" | "SHUTDOWN" | "CONFIG" => return false,
                _ => {}
            }
        }

        // Scan-heavy commands
        if !self.allow_scan_heavy {
            match cmd_upper.as_str() {
                "KEYS" | "RANDOMKEY" | "SCAN" | "SSCAN" | "HSCAN" | "ZSCAN" => return false,
                _ => {}
            }
        }

        true
    }
}

/// A fluent builder for Redis commands.
///
/// Collects command name and arguments, then encodes them into RESP2 wire
/// format when [`build()`](Self::build) is called.
#[derive(Clone)]
pub struct CommandBuilder {
    args: Vec<RedisValue>,
    buf: Vec<Vec<u8>>,
    policy: CommandPolicy,
}

impl CommandBuilder {
    /// Create a new `CommandBuilder` with the given command name.
    ///
    /// The command name is converted to a `BulkString` `RedisValue`.
    /// Uses the default [`CommandPolicy`].
    #[must_use]
    pub fn new(cmd: &str) -> Self {
        Self {
            args: vec![RedisValue::BulkString(cmd.as_bytes().to_vec())],
            buf: Vec::new(),
            policy: CommandPolicy::DEFAULT,
        }
    }

    /// Create a new `CommandBuilder` with a custom policy.
    #[must_use]
    pub fn new_with_policy(cmd: &str, policy: CommandPolicy) -> Self {
        Self {
            args: vec![RedisValue::BulkString(cmd.as_bytes().to_vec())],
            buf: Vec::new(),
            policy,
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

    /// Encode the command into RESP2 wire format using the builder's
    /// current [`CommandPolicy`].
    ///
    /// # Returns
    ///
    /// `Some(BytesMut)` if the command is allowed by the current
    /// [`CommandPolicy`], `None` if it is blocked.
    #[must_use]
    pub fn build(self) -> Option<BytesMut> {
        let cmd_name = self.args.first().and_then(|arg| {
            if let RedisValue::BulkString(data) = arg {
                Some(data.clone())
            } else {
                None
            }
        });
        if let Some(ref name) = cmd_name {
            if let Ok(cmd_str) = std::str::from_utf8(name) {
                if !self.policy.is_allowed(cmd_str) {
                    return None;
                }
            }
        }

        let mut writer = RESPWriter::new();
        writer.write_array_header(self.args.len());
        for arg in &self.args {
            writer.write_value(arg);
        }
        Some(writer.take())
    }

    /// Encode the command into RESP2 wire format using a custom policy.
    ///
    /// # Returns
    ///
    /// `Some(BytesMut)` if the command is allowed by the given
    /// policy, `None` if it is blocked.
    ///
    /// AC-3.11: Commands are validated at build time, not at execution time.
    /// If a command is blocked, this returns `None` and no data is
    /// sent to the connection loop.
    #[must_use]
    pub fn build_with_policy(mut self, policy: CommandPolicy) -> Option<BytesMut> {
        self.policy = policy;
        // AC-3.11: validate command against policy before encoding
        let cmd = self.args.first()?;
        if let RedisValue::BulkString(data) = cmd {
            let cmd_str = std::str::from_utf8(data).ok()?;
            if !self.policy.is_allowed(cmd_str) {
                return None;
            }
        }

        let mut writer = RESPWriter::new();
        writer.write_array_header(self.args.len());
        for arg in &self.args {
            writer.write_value(arg);
        }
        Some(writer.take())
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
        let buf = cmd("SET").arg("k").arg("v").build();
        assert_eq!(
            buf.unwrap().as_ref(),
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"
        );
    }

    #[test]
    fn test_cmd_get_key() {
        let buf = cmd("GET").arg("key").build();
        assert_eq!(buf.unwrap().as_ref(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
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
            buf.unwrap().as_ref(),
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
        assert_eq!(buf.unwrap().as_ref(), b"*2\r\n$4\r\nINCR\r\n$2\r\n42\r\n");
    }

    // ======================== Issue #9 tests ========================

    #[test]
    fn test_command_policy_default_blocks_flushall() {
        let cmd = cmd("FLUSHALL");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_flushdb() {
        let cmd = cmd("FLUSHDB");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_debug() {
        let cmd = cmd("DEBUG");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_shutdown() {
        let cmd = cmd("SHUTDOWN");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_config() {
        let cmd = cmd("CONFIG");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_keys() {
        let cmd = cmd("KEYS");
        assert!(cmd.build().is_none());
    }

    #[test]
    fn test_command_policy_default_blocks_scan_commands() {
        assert!(cmd("RANDOMKEY").build().is_none());
        assert!(cmd("SCAN").build().is_none());
        assert!(cmd("SSCAN").build().is_none());
        assert!(cmd("HSCAN").build().is_none());
        assert!(cmd("ZSCAN").build().is_none());
    }

    #[test]
    fn test_command_policy_default_allows_safe_commands() {
        assert!(cmd("GET").build().is_some());
        assert!(cmd("SET").build().is_some());
        assert!(cmd("DEL").build().is_some());
        assert!(cmd("PING").build().is_some());
    }

    #[test]
    fn test_command_policy_permissive_allows_everything() {
        assert!(cmd("SET")
            .build_with_policy(CommandPolicy::PERMISSIVE)
            .is_some());
        assert!(cmd("FLUSHALL")
            .build_with_policy(CommandPolicy::PERMISSIVE)
            .is_some());
        assert!(cmd("KEYS")
            .build_with_policy(CommandPolicy::PERMISSIVE)
            .is_some());
    }

    #[test]
    fn test_command_policy_case_insensitive() {
        assert!(cmd("flushall").build().is_none());
        assert!(cmd("FlushAll").build().is_none());
        assert!(cmd("FLUSHALL").build().is_none());
    }

    #[test]
    fn test_command_policy_partial_allow_dangerous() {
        let mut p = CommandPolicy::DEFAULT;
        p.allow_dangerous_writes = true;
        let result = cmd("FLUSHALL").build_with_policy(p);
        assert!(result.is_some());
        // But scan-heavy should still be blocked
        assert!(cmd("KEYS").build_with_policy(p).is_none());
    }

    #[test]
    fn test_command_policy_partial_allow_scan_heavy() {
        let mut p = CommandPolicy::DEFAULT;
        p.allow_scan_heavy = true;
        assert!(cmd("KEYS").build_with_policy(p).is_some());
        // But dangerous writes should still be blocked
        assert!(cmd("FLUSHALL").build_with_policy(p).is_none());
    }
}
