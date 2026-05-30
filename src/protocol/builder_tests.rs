// CommandBuilder tests — extract of all #[cfg(test)] code from builder.rs.
// Covers: basic build tests, command_name(), CommandPolicy enum,
// policy deny/allow modes, validate_policy().

use super::builder::{cmd, CommandBuilder, CommandPolicy};

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

    // ======================== command_name() accessor ========================

    #[test]
    fn test_command_name_simple() {
        assert_eq!(cmd("SET").command_name(), Some("SET"));
        assert_eq!(cmd("GET").command_name(), Some("GET"));
    }

    #[test]
    fn test_command_name_case_preserved() {
        assert_eq!(cmd("flushall").command_name(), Some("flushall"));
    }

    // ======================== Issue #9: CommandPolicy enum ========================

    #[test]
    fn test_policy_allow_all() {
        let p = CommandPolicy::AllowAll;
        assert!(p.is_allowed("FLUSHALL"));
        assert!(p.is_allowed("SET"));
        assert!(p.is_allowed("KEYS"));
    }

    #[test]
    fn test_policy_deny_all_blocks_dangerous() {
        // deny_all() blocks: FLUSHALL, FLUSHDB, CONFIG, DEBUG, SLAVEOF, REPLICAOF,
        // SHUTDOWN, KEYS, BGSAVE, BGREWRITEAOF
        let p = CommandPolicy::deny_all();
        assert!(!p.is_allowed("FLUSHALL"));
        assert!(!p.is_allowed("FLUSHDB"));
        assert!(!p.is_allowed("CONFIG"));
        assert!(!p.is_allowed("DEBUG"));
        assert!(!p.is_allowed("SLAVEOF"));
        assert!(!p.is_allowed("REPLICAOF"));
        assert!(!p.is_allowed("SHUTDOWN"));
        assert!(!p.is_allowed("KEYS"));
        assert!(!p.is_allowed("BGSAVE"));
        assert!(!p.is_allowed("BGREWRITEAOF"));
    }

    #[test]
    fn test_command_policy_deny_all_allows_safe() {
        let p = CommandPolicy::deny_all();
        assert!(p.is_allowed("GET"));
        assert!(p.is_allowed("SET"));
        assert!(p.is_allowed("DEL"));
        assert!(p.is_allowed("PING"));
    }

    #[test]
    fn test_policy_allow_set_whitelist() {
        let p = CommandPolicy::allow_set(&["GET", "SET", "DEL"]);
        assert!(p.is_allowed("GET"));
        assert!(p.is_allowed("SET"));
        assert!(p.is_allowed("DEL"));
        assert!(!p.is_allowed("FLUSHALL"));
        assert!(!p.is_allowed("KEYS"));
    }

    #[test]
    fn test_policy_case_insensitive() {
        let p = CommandPolicy::deny_set(&["FLUSHALL"]);
        assert!(!p.is_allowed("FLUSHALL"));
        assert!(!p.is_allowed("flushall"));
        assert!(!p.is_allowed("FlushAll"));
    }

    #[test]
    fn test_policy_default_is_allow_all() {
        let p = CommandPolicy::default();
        assert!(matches!(p, CommandPolicy::AllowAll));
        assert!(p.is_allowed("ANYTHING"));
    }

    #[test]
    fn test_policy_deny_set_from_slice() {
        let p = CommandPolicy::deny_set(&["MYCUSTOM"]);
        assert!(!p.is_allowed("MYCUSTOM"));
        assert!(p.is_allowed("SET"));
    }

    #[test]
    fn test_policy_permissive_alias() {
        let p = CommandPolicy::PERMISSIVE;
        assert!(matches!(p, CommandPolicy::AllowAll));
        assert!(p.is_allowed("FLUSHALL"));
    }

    #[test]
    fn test_deny_all_policy_blocks_flushall() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("FLUSHALL");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_deny_all_policy_blocks_flushdb() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("FLUSHDB");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_deny_all_policy_blocks_debug() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("DEBUG");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_deny_all_policy_blocks_shutdown() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("SHUTDOWN");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_deny_all_policy_blocks_config() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("CONFIG");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_deny_all_policy_blocks_keys() {
        let p = CommandPolicy::deny_all();
        let cmd = cmd("KEYS");
        assert!(cmd.build_with_policy(p).is_none());
    }

    #[test]
    fn test_command_policy_deny_all_blocks_scan_commands() {
        // KEYS is in the default deny list (AC-3.13); SCAN-style commands
        // (RANDOMKEY, SCAN, SSCAN, HSCAN, ZSCAN) were blocked by the old
        // allow_scan_heavy flag but are NOT in AC-3.13's required deny set.
        let p = CommandPolicy::deny_all();
        assert!(!p.is_allowed("KEYS"));
    }

    #[test]
    fn test_command_policy_default_allows_safe_commands() {
        // Default is AllowAll — all safe commands pass
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
        let builder = cmd("flushall").build_with_policy(CommandPolicy::deny_all());
        assert!(builder.is_none());
        let builder = cmd("FlushAll").build_with_policy(CommandPolicy::deny_all());
        assert!(builder.is_none());
        let builder = cmd("FLUSHALL").build_with_policy(CommandPolicy::deny_all());
        assert!(builder.is_none());
    }

    // ======================== validate_policy() ========================

    #[test]
    fn test_validate_policy_allows_safe() {
        let builder = cmd("GET");
        assert!(builder.validate_policy(&CommandPolicy::deny_all()).is_ok());
    }

    #[test]
    fn test_validate_policy_denies_dangerous() {
        let builder = cmd("FLUSHALL");
        assert!(builder.validate_policy(&CommandPolicy::deny_all()).is_err());
    }

    #[test]
    fn test_validate_policy_allows_all_passes() {
        let builder = cmd("FLUSHALL");
        assert!(builder.validate_policy(&CommandPolicy::AllowAll).is_ok());
    }

    #[test]
    fn test_validate_policy_error_message() {
        let builder = cmd("CONFIG");
        let err = builder
            .validate_policy(&CommandPolicy::deny_all())
            .unwrap_err();
        assert!(format!("{err}").contains("CONFIG"));
    }

    #[test]
    fn test_validate_policy_whitelist_blocks_unlisted() {
        let builder = cmd("KEYS");
        let whitelist = CommandPolicy::allow_set(&["GET", "SET"]);
        assert!(builder.validate_policy(&whitelist).is_err());
    }

    #[test]
    fn test_validate_policy_whitelist_allows_listed() {
        let builder = cmd("GET");
        let whitelist = CommandPolicy::allow_set(&["GET", "SET"]);
        assert!(builder.validate_policy(&whitelist).is_ok());
    }
}
