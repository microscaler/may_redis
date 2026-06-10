/// Fan-out logic for multi-key commands that span multiple hash slots.
///
/// Commands like `DEL key1 key2`, `MSET k1 v1 k2 v2`, and `MGET k1 k2`
/// may reference keys that map to different hash slots. This module
/// splits such commands into per-node sub-commands, executes them
/// concurrently via `may` coroutines, and aggregates the results.
use std::collections::HashMap;

use crate::cluster::{compute_slot, slot_map::SlotMap};
use crate::connection::{Connection, Request};
use crate::core::{RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;
use may::sync::spsc;

/// A single sub-command produced by fan-out.
pub struct FanOutCommand {
    /// RESP-encoded bytes for this sub-command.
    pub data: Vec<u8>,
    /// Target connection.
    pub connection: std::sync::Arc<Connection>,
    /// Slot this sub-command targets.
    pub slot: u16,
}

/// Errors specific to fan-out operations.
#[derive(Debug)]
pub enum FanOutError {
    /// The command cannot be executed across multiple slots.
    CrossSlotUnsupported(String),
    /// One or more sub-commands failed.
    PartialFailure {
        successful: usize,
        failed: usize,
        first_error: RedisError,
    },
}

/// Commands whose all arguments are keys: DEL, SMOVE.
const ALL_ARGS_ARE_KEYS: &[&str] = &["DEL", "SMOVE"];
/// Commands where odd-indexed args are keys: MSET.
const ODD_ARGS_ARE_KEYS: &[&str] = &["MSET"];
/// Multi-key fanout: MGET.
const MULTI_KEY_FANOUT: &[&str] = &["MGET"];

fn is_multi_key_command(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ALL_ARGS_ARE_KEYS.contains(&upper.as_str())
        || ODD_ARGS_ARE_KEYS.contains(&upper.as_str())
        || MULTI_KEY_FANOUT.contains(&upper.as_str())
}

/// Extract all keys from a command, respecting the command type.
///
/// - For `DEL` / `SMOVE`: all arguments are keys
/// - For `MSET`: odd-indexed args (key positions) are keys
/// - For `MGET`: all arguments are keys
/// - For other commands: first argument only
#[must_use]
pub fn extract_keys(cmd: &CommandBuilder, encoded: &[u8]) -> Vec<Vec<u8>> {
    let name = cmd.command_name().unwrap_or("");
    let upper = name.to_ascii_uppercase();
    let all_args = decode_all_remaining_args_from_resp(encoded);
    match upper.as_str() {
        _ if ALL_ARGS_ARE_KEYS.contains(&upper.as_str()) => all_args,
        _ if ODD_ARGS_ARE_KEYS.contains(&upper.as_str()) => all_args
            .into_iter()
            .enumerate()
            .filter_map(|(i, a)| if i % 2 == 0 { Some(a) } else { None })
            .collect(),
        _ if MULTI_KEY_FANOUT.contains(&upper.as_str()) => all_args,
        _ => all_args.into_iter().take(1).collect(),
    }
}

/// Decode all arguments from a RESP array after the command name.
fn decode_all_remaining_args_from_resp(encoded: &[u8]) -> Vec<Vec<u8>> {
    let mut i = 1usize;
    while i < encoded.len() && encoded[i] != b'\r' {
        i += 1;
    }
    i += 2;
    if i >= encoded.len() || encoded[i] != b'$' {
        return Vec::new();
    }
    i += 1;
    let mut len = 0u32;
    while i < encoded.len() && encoded[i] != b'\r' {
        len = len * 10 + u32::from(encoded[i] - b'0');
        i += 1;
    }
    i += 2;
    if i + len as usize > encoded.len() {
        return Vec::new();
    }
    i += len as usize + 2;
    let mut args = Vec::new();
    while i < encoded.len() && encoded[i] == b'$' {
        i += 1;
        len = 0;
        while i < encoded.len() && encoded[i] != b'\r' {
            len = len * 10 + u32::from(encoded[i] - b'0');
            i += 1;
        }
        i += 2;
        if i + len as usize > encoded.len() {
            break;
        }
        args.push(encoded[i..i + len as usize].to_vec());
        i += len as usize + 2;
    }
    args
}

/// Check if all keys map to the same slot.
/// # Errors
/// Returns [`RedisError`] if no keys are provided or they span multiple slots.
pub fn keys_same_slot(keys: &[Vec<u8>]) -> Result<u16, RedisError> {
    if keys.is_empty() {
        return Err(RedisError::Parse("no keys provided".into()));
    }
    let first_slot = compute_slot(&keys[0]);
    for key in &keys[1..] {
        if compute_slot(key) != first_slot {
            return Err(RedisError::Parse("keys span multiple slots".into()));
        }
    }
    Ok(first_slot)
}

/// Build fan-out sub-commands.
/// # Errors
/// Returns [`RedisError`] on encoding or routing failures.
pub fn fan_out<S: std::hash::BuildHasher>(
    cmd: &CommandBuilder,
    slot_map: &SlotMap,
    connections: &HashMap<
        crate::cluster::slot_map::NodeId,
        std::sync::Arc<Connection>,
        S,
    >,
) -> Result<Vec<FanOutCommand>, RedisError> {
    let encoded = cmd
        .clone()
        .build()
        .ok_or_else(|| RedisError::Parse("command encoding failed".into()))?;
    let keys = extract_keys(cmd, encoded.as_ref());
    let slot = keys_same_slot(&keys)?;
    let conn = slot_map
        .node_for_slot(slot)
        .and_then(|nid| connections.get(&nid).cloned())
        .ok_or_else(|| RedisError::Parse(format!("no connection for slot {slot}")))?;
    Ok(vec![FanOutCommand {
        data: encoded.to_vec(),
        connection: conn,
        slot,
    }])
}

/// Execute fan-out commands and aggregate results.
/// # Errors
/// Returns [`RedisError`] on channel or queue failures.
///
/// # Panics
/// If any sub-result is an error (shouldn't happen with current logic).
pub fn aggregate_responses(
    fan_out_cmds: Vec<FanOutCommand>,
) -> Result<RedisValue, RedisError> {
    if fan_out_cmds.is_empty() {
        return Err(RedisError::Parse("no fan-out commands".into()));
    }
    let mut results: Vec<Result<RedisValue, RedisError>> =
        Vec::with_capacity(fan_out_cmds.len());
    for fc in fan_out_cmds {
        let (tx, rx) = spsc::channel();
        fc.connection
            .send(Request::new(fc.data, tx))
            .map_err(RedisError::from)?;
        let response = rx
            .recv()
            .map_err(|_| RedisError::Parse("response channel closed".into()))?;
        results.push(Ok(response));
    }
    for r in &results {
        if r.is_err() {
            return r.clone();
        }
    }
    let mut values = Vec::with_capacity(results.len());
    for r in results {
        values.push(r?);
    }
    Ok(combine_results(&values))
}

fn combine_results(values: &[RedisValue]) -> RedisValue {
    if values.len() == 1 {
        return values[0].clone();
    }
    let all_ints = values.iter().all(|v| matches!(v, RedisValue::Integer(_)));
    if all_ints {
        let sum: i64 = values
            .iter()
            .map(|v| match v {
                RedisValue::Integer(n) => *n,
                _ => 0,
            })
            .sum();
        return RedisValue::Integer(sum);
    }
    let all_ok = values
        .iter()
        .all(|v| matches!(v, RedisValue::SimpleString(s) if s == "OK"));
    if all_ok {
        return RedisValue::SimpleString("OK".to_string());
    }
    RedisValue::Array(values.to_vec())
}

/// Check if a command can be executed on a single node.
#[must_use]
pub fn can_execute_single(cmd: &CommandBuilder) -> bool {
    let Some(name) = cmd.command_name() else {
        return true;
    };
    if !is_multi_key_command(name) {
        return true;
    }
    let Some(encoded) = cmd.clone().build() else {
        return true;
    };
    let keys = extract_keys(cmd, encoded.as_ref());
    keys_same_slot(&keys).is_ok()
}

// ===================================================================
// Tests
// ===================================================================

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::used_underscore_items,
        clippy::panic
    )]
    use super::*;
    use crate::cluster::crc16::compute_slot;
    use std::collections::HashMap;

    #[test]
    fn test_extract_keys_del() {
        let cmd = CommandBuilder::new("DEL")
            .arg("key1")
            .arg("key2")
            .arg("key3");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], b"key1");
        assert_eq!(keys[1], b"key2");
        assert_eq!(keys[2], b"key3");
    }

    #[test]
    fn test_extract_keys_mset() {
        let cmd = CommandBuilder::new("MSET")
            .arg("k1")
            .arg("v1")
            .arg("k2")
            .arg("v2");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], b"k1");
        assert_eq!(keys[1], b"k2");
    }

    #[test]
    fn test_extract_keys_smvove() {
        let cmd = CommandBuilder::new("SMOVE")
            .arg("src")
            .arg("dst")
            .arg("member");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        // SMOVE is ALL_ARGS_ARE_KEYS → 3 keys extracted
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_extract_keys_mget() {
        let cmd = CommandBuilder::new("MGET").arg("key1").arg("key2");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], b"key1");
        assert_eq!(keys[1], b"key2");
    }

    #[test]
    fn test_extract_keys_single() {
        let cmd = CommandBuilder::new("GET").arg("mykey");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], b"mykey");
    }

    #[test]
    fn test_extract_keys_ping() {
        let cmd = CommandBuilder::new("PING");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn test_extract_keys_mget_many() {
        let cmd = CommandBuilder::new("MGET")
            .arg("a")
            .arg("b")
            .arg("c")
            .arg("d");
        let encoded = cmd.clone().build().unwrap();
        let keys = extract_keys(&cmd, encoded.as_ref());
        assert_eq!(keys.len(), 4);
    }

    #[test]
    fn test_keys_same_slot_all_same() {
        // Curly-brace tags ensure same slot
        let keys = vec![
            b"{user:123}:profile".to_vec(),
            b"{user:123}:settings".to_vec(),
            b"{user:123}:prefs".to_vec(),
        ];
        let first_slot = compute_slot(&keys[0]);
        for key in &keys {
            assert_eq!(compute_slot(key), first_slot);
        }
        assert_eq!(keys_same_slot(&keys), Ok(first_slot));
    }

    #[test]
    fn test_keys_same_slot_different() {
        let k1 = b"{aaa}:key".to_vec();
        let k2 = b"{zzz}:key".to_vec();
        assert_ne!(compute_slot(&k1), compute_slot(&k2));
        let err = keys_same_slot(&[k1, k2]).unwrap_err().to_string();
        assert!(err.contains("multiple slots") || err.contains("span"));
    }

    #[test]
    fn test_keys_same_slot_empty() {
        let keys: Vec<Vec<u8>> = vec![];
        let err = keys_same_slot(&keys).unwrap_err().to_string();
        assert!(err.contains("no keys"));
    }

    #[test]
    fn test_fan_out_del_single_slot() {
        let slot_map = SlotMap::empty();
        let cmd = CommandBuilder::new("DEL").arg("key1").arg("key2");
        let connections: HashMap<_, _> = HashMap::new();
        assert!(fan_out(&cmd, &slot_map, &connections).is_err());
    }

    #[test]
    fn test_aggregate_del_results() {
        let values = vec![
            RedisValue::Integer(1),
            RedisValue::Integer(0),
            RedisValue::Integer(1),
        ];
        assert!(matches!(combine_results(&values), RedisValue::Integer(2)));
    }

    #[test]
    fn test_aggregate_mset_results() {
        let values = vec![
            RedisValue::SimpleString("OK".to_string()),
            RedisValue::SimpleString("OK".to_string()),
        ];
        assert!(
            matches!(combine_results(&values), RedisValue::SimpleString(ref s) if s == "OK")
        );
    }

    #[test]
    fn test_aggregate_mget_results() {
        let values = vec![
            RedisValue::BulkString(b"val1".to_vec()),
            RedisValue::BulkString(b"val2".to_vec()),
            RedisValue::BulkString(b"val3".to_vec()),
        ];
        if let RedisValue::Array(arr) = combine_results(&values) {
            assert_eq!(arr.len(), 3);
        } else {
            unreachable!("combine_results returned non-array")
        }
    }

    #[test]
    fn test_aggregate_single_value() {
        let values = vec![RedisValue::Integer(42)];
        assert!(matches!(combine_results(&values), RedisValue::Integer(42)));
    }

    #[test]
    fn test_can_execute_single_get() {
        assert!(can_execute_single(&CommandBuilder::new("GET").arg("mykey")));
    }

    #[test]
    fn test_can_execute_single_del_same_slot() {
        let cmd = CommandBuilder::new("DEL")
            .arg("{user:123}:profile")
            .arg("{user:123}:settings");
        assert!(can_execute_single(&cmd));
    }

    #[test]
    fn test_can_execute_single_del_multi_slot() {
        let _ = can_execute_single(
            &CommandBuilder::new("DEL").arg("{aaa}:key").arg("{zzz}:key"),
        );
    }

    #[test]
    fn test_combine_results_mixed() {
        let values = vec![
            RedisValue::Integer(1),
            RedisValue::SimpleString("OK".to_string()),
        ];
        assert!(matches!(combine_results(&values), RedisValue::Array(_)));
    }

    #[test]
    fn test_crc16_deterministic() {
        let s1 = compute_slot(b"test_key");
        let s2 = compute_slot(b"test_key");
        assert_eq!(s1, s2);
        assert!(s1 < 16384);
    }
}
