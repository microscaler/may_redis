// CommandBuilder — Fluent API for building Redis commands.

use crate::codec::writer::RESPWriter;
use crate::core::{RedisValue, ToRedisArgs};
use bytes::BytesMut;
use std::collections::HashSet;

/// Policy for controlling which Redis commands are allowed.
///
/// AC-3.11: Commands are validated at build time, not at execution time.
/// If a command is blocked, [`build`](CommandBuilder::build) returns
/// `None` and no data is sent to the connection loop.
///
/// AC-3.12: `AllowAll` is the default and allows every command (backward
/// compatible). Security-conscious callers should use `DenyCommands`
/// to block dangerous commands like FLUSHALL, CONFIG, DEBUG, SHUTDOWN, etc.
///
/// AC-3.14: `AllowCommands` provides a whitelist mode — only the specified
/// commands pass validation.
///
/// NFR-015: All three variants use `HashSet` for O(1) command lookups.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommandPolicy {
    /// Allow all commands (no restrictions).
    ///
    /// Default for backward compatibility (AC-3.12). Documented as a
    /// security concern — callers should prefer `DenyCommands`.
    #[default]
    AllowAll,

    /// Deny the listed commands; allow everything else.
    DenyCommands(HashSet<String>),

    /// Allow only the listed commands; deny everything else.
    AllowCommands(HashSet<String>),
}

/// Default set of dangerous commands denied by `deny_all()`.
///
/// FLUSHALL, FLUSHDB, CONFIG, DEBUG, SLAVEOF, REPLICAOF, SHUTDOWN,
/// KEYS, BGSAVE, BGREWRITEAOF.
const DEFAULT_DENY_SET: &[&str] = &[
    "FLUSHALL",
    "FLUSHDB",
    "CONFIG",
    "DEBUG",
    "SLAVEOF",
    "REPLICAOF",
    "SHUTDOWN",
    "KEYS",
    "BGSAVE",
    "BGREWRITEAOF",
];

/// Lazily-initialized HashSet for the default deny list.
static DEFAULT_DENY_HASHSET: std::sync::LazyLock<std::collections::HashSet<String>> =
    std::sync::LazyLock::new(|| {
        DEFAULT_DENY_SET
            .iter()
            .map(|s| s.to_ascii_uppercase())
            .collect()
    });

impl CommandPolicy {
    /// Permissive policy: allow all commands.
    pub const PERMISSIVE: Self = Self::AllowAll;

    /// Strict policy: deny all dangerous commands (AC-3.13).
    ///
    /// Blocks: FLUSHALL, FLUSHDB, CONFIG, DEBUG, SLAVEOF, REPLICAOF,
    /// SHUTDOWN, KEYS, BGSAVE, BGREWRITEAOF.
    #[must_use]
    pub fn deny_all() -> Self {
        Self::DenyCommands((*DEFAULT_DENY_HASHSET).clone())
    }

    /// Create a deny policy from a slice of command names.
    ///
    /// Command names are stored in uppercase for case-insensitive matching.
    #[must_use]
    pub fn deny_set(cmds: &[&str]) -> Self {
        let set = cmds.iter().map(|s| s.to_ascii_uppercase()).collect();
        Self::DenyCommands(set)
    }

    /// Create a whitelist (allow) policy from a list of command names.
    ///
    /// Command names are stored in uppercase for case-insensitive matching.
    #[must_use]
    pub fn allow_set(cmds: &[&str]) -> Self {
        let set = cmds.iter().map(|s| s.to_ascii_uppercase()).collect();
        Self::AllowCommands(set)
    }

    /// Check if a command is allowed by this policy.
    #[must_use]
    pub fn is_allowed(&self, cmd: &str) -> bool {
        let cmd_upper = cmd.to_ascii_uppercase();
        match self {
            Self::AllowAll => true,
            Self::DenyCommands(denied) => !denied.contains(&cmd_upper),
            Self::AllowCommands(allowed) => allowed.contains(&cmd_upper),
        }
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
    /// Uses the default [`CommandPolicy::AllowAll`].
    #[must_use]
    pub fn new(cmd: &str) -> Self {
        Self {
            args: vec![RedisValue::BulkString(cmd.as_bytes().to_vec())],
            buf: Vec::new(),
            policy: CommandPolicy::default(),
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

    /// Returns the command name as a UTF-8 string.
    ///
    /// FR-032: Accessor for policy checks. Returns `None` if the
    /// command name is not valid UTF-8.
    #[must_use]
    pub fn command_name(&self) -> Option<&str> {
        if let RedisValue::BulkString(data) = self.args.first()? {
            std::str::from_utf8(data).ok()
        } else {
            None
        }
    }

    /// Validate this command against a [`CommandPolicy`].
    ///
    /// FR-030: Returns `Ok(())` if the command is allowed by the policy,
    /// `Err(RedisError::Security)` if it is denied.
    ///
    /// AC-3.11: Policy checking happens here, before the command is
    /// encoded or sent to the connection loop.
    ///
    /// # Errors
    /// Returns [`RedisError::Security`] if the command is blocked
    /// by the given policy.
    pub fn validate_policy(&self, policy: &CommandPolicy) -> Result<(), crate::core::RedisError> {
        if let Some(name) = self.command_name() {
            if !policy.is_allowed(name) {
                return Err(crate::core::RedisError::Security(format!(
                    "command '{name}' is denied by policy"
                )));
            }
        }
        Ok(())
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
