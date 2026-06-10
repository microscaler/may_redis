/// Redis Cluster redirect handling — MOVED and ASK responses.
///
/// When a cluster node receives a command for a slot it no longer owns,
/// it responds with `MOVED slot addr`. During resharding, it may respond
/// with `ASK slot addr` which also requires sending `ASKING` before retry.
use std::net::SocketAddr;

use crate::cluster::slot_map::SlotMap;
use crate::connection::Connection;
use crate::core::{RedisError, RedisValue};

// ---------------------------------------------------------------------------
// RedirectKind + Redirect
// ---------------------------------------------------------------------------

/// Type of cluster redirect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectKind {
    /// Permanent move — slot ownership changed permanently.
    Moved,
    /// Temporary move — resharding in progress.
    Ask,
}

/// A redirect response from Redis Cluster.
#[derive(Debug, Clone)]
pub struct Redirect {
    /// The hash slot that was redirected.
    pub slot: u16,
    /// The node address to retry on (host:port).
    pub target: SocketAddr,
    /// Whether this is a MOVED (permanent) or ASK (temporary) redirect.
    pub kind: RedirectKind,
}

impl std::fmt::Display for Redirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} -> {}",
            match self.kind {
                RedirectKind::Moved => "MOVED",
                RedirectKind::Ask => "ASK",
            },
            self.slot,
            self.target
        )
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a MOVED redirect from a RedisValue.
///
/// Redis returns MOVED as a RESP error bulk string: `MOVED 3999 192.168.1.20:6380`.
///
/// # Errors
/// Returns [`RedisError::Parse`] if the value is not a MOVED redirect.
#[must_use]
pub fn parse_moved_redirect(value: &RedisValue) -> Option<Redirect> {
    let RedisValue::Error(ref msg) = value else {
        return None;
    };
    parse_redirect(msg, RedirectKind::Moved)
}

/// Parse an ASK redirect from a RedisValue.
///
/// Redis returns ASK as a RESP error bulk string: `ASK 3999 192.168.1.20:6379`.
///
/// # Errors
/// Returns [`RedisError::Parse`] if the value is not an ASK redirect.
#[must_use]
pub fn parse_ask_redirect(value: &RedisValue) -> Option<Redirect> {
    let RedisValue::Error(ref msg) = value else {
        return None;
    };
    parse_redirect(msg, RedirectKind::Ask)
}

/// Generic redirect parser for MOVED and ASK.
fn parse_redirect(text: &str, kind: RedirectKind) -> Option<Redirect> {
    let expected_prefix = match kind {
        RedirectKind::Moved => "MOVED ",
        RedirectKind::Ask => "ASK ",
    };
    if !text.starts_with(expected_prefix) {
        return None;
    }

    // Split into three parts: TYPE SLOT ADDR
    let mut parts = text.splitn(3, ' ');
    let _prefix = parts.next()?; // "MOVED" or "ASK"
    let slot_str = parts.next()?;
    let addr_str = parts.next()?;

    let slot: u16 = slot_str.parse().ok()?;
    let target: SocketAddr = addr_str.parse().ok()?;

    Some(Redirect { slot, target, kind })
}

// ---------------------------------------------------------------------------
// SlotMap helper: update on redirect
// ---------------------------------------------------------------------------

/// Update a slot map when a MOVED or ASK redirect is received.
///
/// Reassigns the redirected slot to the target node so the next command
/// using the same key goes directly to the correct node.
///
/// # Returns
/// `true` if the slot mapping was updated. `false` (with a warning log)
/// if the redirect target is not a known node — the map is then stale
/// and the caller should refresh the cluster topology; retries against
/// the stale map will keep redirecting until the retry limit surfaces
/// an error.
pub fn update_slot_map_on_redirect(map: &mut SlotMap, redirect: &Redirect) -> bool {
    let target_id = map
        .nodes()
        .find(|node| node.addr == redirect.target)
        .map(|node| node.id);
    let Some(node_id) = target_id else {
        log::warn!(
            "redirect target {} is not a known cluster node; \
             slot {} mapping left stale (topology refresh needed)",
            redirect.target,
            redirect.slot
        );
        return false;
    };
    map.assign_slot(redirect.slot, node_id);
    true
}

// ---------------------------------------------------------------------------
// Retry helper
// ---------------------------------------------------------------------------

/// Retry a command after a redirect.
///
/// # Arguments
/// * `conn` — The connection to retry on.
/// * `ask` — Whether to send ASKING before the command (for ASK redirects).
/// * `encoded` — The pre-encoded RESP bytes to send.
///
/// # May runtime requirement
/// Requires the may coroutine runtime.
///
/// # Errors
/// Returns [`RedisError::Connection`] if the command cannot be queued on
/// the connection, and [`RedisError::Parse`] on protocol errors.
pub fn retry_command(
    conn: &Connection,
    ask: bool,
    encoded: &[u8],
) -> Result<RedisValue, RedisError> {
    use may::sync::spsc;

    if ask {
        // Send ASKING first
        let asking = crate::protocol::builder::CommandBuilder::new("ASKING")
            .build()
            .ok_or_else(|| RedisError::Parse("ASKING encoding failed".into()))?;
        let (tx, rx) = spsc::channel();
        conn.send(crate::connection::Request::new(asking.to_vec(), tx))
            .map_err(|e| RedisError::Connection(format!("ASKING send failed: {e}")))?;
        let _resp = rx
            .recv()
            .map_err(|_| RedisError::Parse("ASKING response channel closed".into()))?;
    }

    // Send the original command
    let (tx, rx) = spsc::channel();
    conn.send(crate::connection::Request::new(encoded.to_vec(), tx))
        .map_err(|e| RedisError::Connection(format!("retry send failed: {e}")))?;

    rx.recv()
        .map_err(|_| RedisError::Parse("retry response channel closed".into()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn test_parse_moved_valid() {
        let value = RedisValue::Error("MOVED 3999 192.168.1.20:6380".to_string());
        let redirect = parse_moved_redirect(&value);
        assert!(redirect.is_some());
        let r = redirect.unwrap();
        assert_eq!(r.slot, 3999);
        assert_eq!(r.target, "192.168.1.20:6380".parse().unwrap());
        assert_eq!(r.kind, RedirectKind::Moved);
    }

    #[test]
    fn test_parse_ask_valid() {
        let value = RedisValue::Error("ASK 3999 192.168.1.20:6379".to_string());
        let redirect = parse_ask_redirect(&value);
        assert!(redirect.is_some());
        let r = redirect.unwrap();
        assert_eq!(r.slot, 3999);
        assert_eq!(r.target, "192.168.1.20:6379".parse().unwrap());
        assert_eq!(r.kind, RedirectKind::Ask);
    }

    #[test]
    fn test_parse_non_redirect() {
        let value = RedisValue::Error("ERR unknown command".to_string());
        assert!(parse_moved_redirect(&value).is_none());
        assert!(parse_ask_redirect(&value).is_none());
    }

    #[test]
    fn test_parse_wrong_kind() {
        let value = RedisValue::Error("MOVED 100 10.0.0.1:6379".to_string());
        // ASK parser should reject MOVED
        assert!(parse_ask_redirect(&value).is_none());
        // MOVED parser should accept it
        assert!(parse_moved_redirect(&value).is_some());
    }

    #[test]
    fn test_parse_invalid_slot() {
        let value = RedisValue::Error("MOVED abc 10.0.0.1:6379".to_string());
        assert!(parse_moved_redirect(&value).is_none());
    }

    #[test]
    fn test_parse_invalid_address() {
        let value = RedisValue::Error("MOVED 100 notanaddress".to_string());
        assert!(parse_moved_redirect(&value).is_none());
    }

    #[test]
    fn test_parse_simple_string_not_error() {
        let value = RedisValue::SimpleString("MOVED 100 10.0.0.1:6379".to_string());
        assert!(parse_moved_redirect(&value).is_none());
    }

    #[test]
    fn test_moved_update_slot_map() {
        let mut map = SlotMap::empty();
        // Add two nodes: old node (nodeA) and redirect target (nodeB)
        let node_a = crate::cluster::slot_map::NodeInfo {
            id: crate::cluster::slot_map::NodeId::from_hex(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            addr: "10.0.0.1:6379".parse().unwrap(),
            role: crate::cluster::slot_map::NodeRole::Master,
            slots: Some(0..=100),
            state: crate::cluster::slot_map::NodeState::Online,
        };
        let node_b = crate::cluster::slot_map::NodeInfo {
            id: crate::cluster::slot_map::NodeId::from_hex(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            addr: "10.0.0.2:6379".parse().unwrap(),
            role: crate::cluster::slot_map::NodeRole::Master,
            slots: Some(101..=200),
            state: crate::cluster::slot_map::NodeState::Online,
        };
        map.add_node(node_a);
        map.add_node(node_b);

        let redirect = Redirect {
            slot: 50,
            target: "10.0.0.2:6379".parse().unwrap(),
            kind: RedirectKind::Moved,
        };
        let updated = update_slot_map_on_redirect(&mut map, &redirect);

        // Positive: slot 50 must actually move to node B (previously the
        // function re-added node B's own range and left slot 50 on node A).
        assert!(updated, "known target must report an update");
        assert_eq!(
            map.node_for_slot(50),
            Some(crate::cluster::slot_map::NodeId::from_hex(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            )),
            "redirected slot must point at the target node"
        );
        assert!(!map.is_empty());
    }

    /// Negative: a redirect to a node the map does not know is reported
    /// (returns false) instead of silently doing nothing.
    #[test]
    fn test_moved_update_slot_map_unknown_target() {
        let mut map = SlotMap::empty();
        let node_a = crate::cluster::slot_map::NodeInfo {
            id: crate::cluster::slot_map::NodeId::from_hex(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            addr: "10.0.0.1:6379".parse().unwrap(),
            role: crate::cluster::slot_map::NodeRole::Master,
            slots: Some(0..=100),
            state: crate::cluster::slot_map::NodeState::Online,
        };
        map.add_node(node_a);

        let redirect = Redirect {
            slot: 50,
            target: "10.0.0.99:6379".parse().unwrap(), // not in the map
            kind: RedirectKind::Moved,
        };
        let updated = update_slot_map_on_redirect(&mut map, &redirect);

        assert!(!updated, "unknown target must report no update");
        assert_eq!(
            map.node_for_slot(50),
            Some(crate::cluster::slot_map::NodeId::from_hex(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            )),
            "slot ownership must be unchanged"
        );
    }
}
