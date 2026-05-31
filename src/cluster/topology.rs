/// CLUSTER NODES and CLUSTER SLOTS response parsing.
///
/// Parses the textual output of `CLUSTER NODES` and the array output of
/// `CLUSTER SLOTS` into `SlotMap` / `NodeInfo` data structures.
use std::net::SocketAddr;

use crate::cluster::slot_map::{NodeInfo, NodeRole, NodeState, SlotMap};
use crate::core::{FromRedisValue, RedisError, RedisValue};

// ---------------------------------------------------------------------------
// Node metadata (Story 3.1: NodeBusInfo, NodeFlag, NodeLinkState)
// ---------------------------------------------------------------------------

/// Parsed flags from CLUSTER NODES output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeFlag {
    /// This node is the one sending the CLUSTER NODES message.
    Myself,
    /// Node accepts reads and writes (master).
    Master,
    /// Node is a replica.
    Slave,
    /// Node is believed to be unreachable (fail mark).
    Fail,
    /// Node is possibly unreachable (pfail mark).
    Pfail,
}

/// Link state from CLUSTER NODES output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeLinkState {
    /// Node bus connection is up.
    Connected,
    /// Node bus connection is down.
    Disconnected,
}

/// Information about a cluster node's bus and status.
#[derive(Debug, Clone)]
pub struct NodeBusInfo {
    /// Bus port (Gossip protocol).
    pub bus_port: u16,
    /// Flags parsed from the flags field.
    pub flags: Vec<NodeFlag>,
    /// Master node ID (None for masters).
    pub master_id: Option<NodeId>,
    /// Last sent ping timestamp (ms).
    pub ping_sent: u64,
    /// Last received pong timestamp (ms).
    pub pong_received: u64,
    /// Configuration epoch.
    pub config_epoch: u64,
    /// Bus link state.
    pub link_state: NodeLinkState,
}

use crate::cluster::slot_map::NodeId;

// ---------------------------------------------------------------------------
// CLUSTER NODES parsing
// ---------------------------------------------------------------------------

/// Parse a `CLUSTER NODES` response into a SlotMap.
///
/// The CLUSTER NODES response format (one line per node):
/// ```text
/// <node-id> <addr>:<port>@<bus-port> <flags> <master-id> <ping-sent>
/// <pong-received> <config-epoch> <link-state> <slot> ... <slot>
/// ```
///
/// Flags include: `myself`, `master`, `slave`, `fail`, `pfail`.
/// Slots are listed as ranges (`0-5460`) or single slots.
///
/// # Errors
/// Returns [`RedisError::Parse`] if the response is malformed.
///
/// # Panics
/// Does not panic — all parsing errors are returned as [`RedisError`].
pub fn parse_cluster_nodes(text: &str) -> Result<SlotMap, RedisError> {
    let mut map = SlotMap::empty();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return Err(RedisError::Parse(format!(
                "malformed CLUSTER NODES entry (too few fields: {n}, need >=8): {line}",
                n = parts.len()
            )));
        }

        let node_id_str = parts[0];
        let addr_str = parts[1];
        let flags_str = parts[2];

        // Parse address: host:port@bus-port
        let (host_port, bus_port_str) = addr_str.rsplit_once('@').ok_or_else(|| {
            RedisError::Parse(format!("malformed address: {addr_str}"))
        })?;
        let (host, port_str) = host_port.rsplit_once(':').ok_or_else(|| {
            RedisError::Parse(format!("malformed host:port: {host_port}"))
        })?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| RedisError::Parse(format!("invalid port: {port_str}")))?;
        let bus_port: u16 = bus_port_str.parse().map_err(|_| {
            RedisError::Parse(format!("invalid bus port: {bus_port_str}"))
        })?;
        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| RedisError::Parse(format!("invalid address: {addr_str}")))?;

        // Parse flags.
        let is_master = flags_str.contains("master") && !flags_str.contains("slave");
        let is_fail = flags_str.contains("fail") || flags_str.contains("pfail");
        let role = if is_master {
            NodeRole::Master
        } else {
            NodeRole::Replica
        };
        let state = if is_fail {
            NodeState::Down
        } else {
            NodeState::Online
        };

        // Parse bus info (fields 3-8).
        let master_id = if parts.len() > 3 && parts[3] != "-" {
            Some(NodeId::from_hex(parts[3]))
        } else {
            None
        };
        let ping_sent: u64 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let pong_received: u64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let config_epoch: u64 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let link_state = match parts.get(7).map(|s| *s) {
            Some("connected") => NodeLinkState::Connected,
            Some("disconnected") => NodeLinkState::Disconnected,
            Some(s) => {
                return Err(RedisError::Parse(format!("unknown link state: {s}")))
            }
            None => NodeLinkState::Disconnected,
        };

        let flags: Vec<NodeFlag> = flags_str
            .split(',')
            .filter_map(|f| match f {
                "myself" => Some(NodeFlag::Myself),
                "master" => Some(NodeFlag::Master),
                "slave" | "slave?replica" => Some(NodeFlag::Slave),
                "fail" => Some(NodeFlag::Fail),
                "pfail" => Some(NodeFlag::Pfail),
                _ => None,
            })
            .collect();

        // Parse slot ranges (remaining fields from index 8).
        let slot_parts = &parts[8..];
        let mut slots: Vec<u16> = Vec::new();
        for slot_spec in slot_parts {
            if slot_spec.contains('-') {
                let mut range = slot_spec.split('-');
                let start_str = range.next().ok_or_else(|| {
                    RedisError::Parse(format!("invalid slot range: {slot_spec}"))
                })?;
                let end_str = range.next().ok_or_else(|| {
                    RedisError::Parse(format!("invalid slot range: {slot_spec}"))
                })?;
                let start: u16 = start_str.parse().map_err(|_| {
                    RedisError::Parse(format!(
                        "invalid slot start in range: {slot_spec}"
                    ))
                })?;
                let end: u16 = end_str.parse().map_err(|_| {
                    RedisError::Parse(format!("invalid slot end in range: {slot_spec}"))
                })?;
                if start <= end {
                    slots.extend(start..=end);
                }
            } else if !slot_spec.is_empty() {
                let slot: u16 = slot_spec.parse().map_err(|_| {
                    RedisError::Parse(format!("invalid slot: {slot_spec}"))
                })?;
                slots.push(slot);
            }
        }

        let node = NodeInfo {
            id: NodeId::from_hex(node_id_str),
            addr,
            role,
            state,
            slots: if slots.is_empty() {
                None
            } else {
                Some(*slots.first().unwrap()..=*slots.last().unwrap())
            },
        };

        let _bus_info = NodeBusInfo {
            bus_port,
            flags,
            master_id,
            ping_sent,
            pong_received,
            config_epoch,
            link_state,
        };

        map.add_node(node);
    }

    Ok(map)
}

// ---------------------------------------------------------------------------
// CLUSTER SLOTS parsing
// ---------------------------------------------------------------------------

/// Parse a `CLUSTER SLOTS` response into a SlotMap.
///
/// The CLUSTER SLOTS response is a nested array:
/// ```text
/// *1
///   *3
///     :0
///     :5460
///     *3
///       $10
///       192.168.1.1
///       :6379
///       $40
///       a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0
///     *2
///       $10
///       192.168.1.2
///       :6380
///       $40
///       b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0
/// ```
///
/// # Errors
/// Returns [`RedisError::Parse`] if the response is not an array or entries are malformed.
///
/// # Panics
/// Does not panic — all parsing errors are returned as [`RedisError`].
pub fn parse_cluster_slots(value: &RedisValue) -> Result<SlotMap, RedisError> {
    let RedisValue::Array(entries) = value else {
        return Err(RedisError::Parse(format!(
            "CLUSTER SLOTS response is not an array, got {value:?}"
        )));
    };

    let mut map = SlotMap::empty();

    for entry in entries {
        let RedisValue::Array(fields) = entry else {
            return Err(RedisError::Parse(
                "CLUSTER SLOTS entry is not an array".into(),
            ));
        };

        if fields.len() < 3 {
            return Err(RedisError::Parse(
                "CLUSTER SLOTS entry has too few fields".into(),
            ));
        }

        // Parse start/end slots.
        let start_slot: i64 = FromRedisValue::from_redis_value(&fields[0])?;
        let end_slot: i64 = FromRedisValue::from_redis_value(&fields[1])?;
        let start_slot = start_slot as u16;
        let end_slot = end_slot as u16;

        // Parse primary node (index 2 is an array: [ip, port, node-id?]).
        let RedisValue::Array(primary_fields) = &fields[2] else {
            return Err(RedisError::Parse(
                "CLUSTER SLOTS primary node is not an array".into(),
            ));
        };

        if primary_fields.len() < 2 {
            return Err(RedisError::Parse(
                "CLUSTER SLOTS primary node has too few fields".into(),
            ));
        }

        let ip: String = FromRedisValue::from_redis_value(&primary_fields[0])?;
        let port_raw: i64 = FromRedisValue::from_redis_value(&primary_fields[1])?;
        let port: u16 = port_raw as u16;
        let addr = format!("{ip}:{port}").parse().map_err(|_| {
            RedisError::Parse(format!("invalid primary address: {ip}:{port}"))
        })?;

        let node_id = if primary_fields.len() > 2 {
            let raw: String = FromRedisValue::from_redis_value(&primary_fields[2])?;
            if raw.is_empty() || raw == "-" {
                None
            } else {
                Some(NodeId::from_hex(&raw))
            }
        } else {
            None
        };

        let node_id_val = node_id.unwrap_or_else(|| {
            NodeId::from_hex("0000000000000000000000000000000000000000")
        });

        let node = NodeInfo {
            id: node_id_val,
            addr,
            role: NodeRole::Master,
            state: NodeState::Online,
            slots: Some(start_slot..=end_slot),
        };

        map.add_node(node);

        // Parse replicas (remaining fields 3..).
        for replica_field in fields.iter().skip(3) {
            let RedisValue::Array(replica_fields) = replica_field else {
                return Err(RedisError::Parse(
                    "CLUSTER SLOTS replica is not an array".into(),
                ));
            };

            if replica_fields.len() < 2 {
                return Err(RedisError::Parse(
                    "CLUSTER SLOTS replica has too few fields".into(),
                ));
            }

            let rip: String = FromRedisValue::from_redis_value(&replica_fields[0])?;
            let rport_raw: i64 = FromRedisValue::from_redis_value(&replica_fields[1])?;
            let rport: u16 = rport_raw as u16;
            let raddr = format!("{rip}:{rport}").parse().map_err(|_| {
                RedisError::Parse(format!("invalid replica address: {rip}:{rport}"))
            })?;

            let replica_id = if replica_fields.len() > 2 {
                let raw: String = FromRedisValue::from_redis_value(&replica_fields[2])?;
                if raw.is_empty() || raw == "-" {
                    None
                } else {
                    Some(NodeId::from_hex(&raw))
                }
            } else {
                None
            };

            let replica_id_val = replica_id.unwrap_or_else(|| {
                NodeId::from_hex("0000000000000000000000000000000000000001")
            });

            let replica = NodeInfo {
                id: replica_id_val,
                addr: raddr,
                role: NodeRole::Replica,
                state: NodeState::Online,
                slots: None,
            };

            map.add_node(replica);
        }
    }

    Ok(map)
}

// ---------------------------------------------------------------------------
// CLUSTERDOWN detection
// ---------------------------------------------------------------------------

/// Check if a RedisValue indicates a CLUSTERDOWN error.
///
/// Returns `Some(slot)` if the error indicates a slot is unassigned.
pub fn parse_clusterdown(value: &RedisValue) -> Option<u16> {
    let RedisValue::Error(ref msg) = value else {
        return None;
    };
    if !msg.starts_with("-CLUSTERDOWN ") {
        return None;
    }
    // Try to extract the slot from "CLUSTERDOWN The cluster is down - hash slot 3999 not owned"
    let after = &msg["-CLUSTERDOWN ".len()..];
    // Find the slot number in the message.
    after
        .split_whitespace()
        .find(|s| s.chars().all(|c| c.is_ascii_digit()))
        .and_then(|s| s.parse().ok())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::used_underscore_items
    )]

    use super::*;
    use crate::cluster::slot_map::NodeId;

    /// Helper: create a NodeId from hex.
    fn mknodeid(s: &str) -> NodeId {
        NodeId::from_hex(s)
    }

    /// Test: parse a standard 3-master CLUSTER NODES response.
    ///
    /// Expected: 3 nodes with correct slot ranges (0-5460, 5461-10922, 10923-16383).
    #[test]
    fn test_parse_cluster_nodes_3_masters() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 master - 0 1700000000000 1 connected 0-5460\n\
bbb000bbb000bbb000bbb000bbb000bbb000 192.168.1.11:6379@16379 master - 0 1700000000000 2 connected 5461-10922\n\
ccc000ccc000ccc000ccc000ccc000ccc000 192.168.1.12:6379@16379 master - 0 1700000000000 3 connected 10923-16383\n";

        let map = parse_cluster_nodes(text).unwrap();

        assert_eq!(map.len(), 3);
        assert!(map.is_complete());

        // Verify first node owns slots 0-5460
        assert_eq!(
            map.node_for_slot(0),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(5460),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );

        // Verify second node owns slots 5461-10922
        assert_eq!(
            map.node_for_slot(5461),
            Some(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"))
        );
        assert_eq!(
            map.node_for_slot(10922),
            Some(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"))
        );

        // Verify third node owns slots 10923-16383
        assert_eq!(
            map.node_for_slot(10923),
            Some(mknodeid("ccc000ccc000ccc000ccc000ccc000ccc000"))
        );
        assert_eq!(
            map.node_for_slot(16383),
            Some(mknodeid("ccc000ccc000ccc000ccc000ccc000ccc000"))
        );

        // Verify boundary nodes are different
        assert_ne!(map.node_for_slot(10922), map.node_for_slot(10923));
    }

    /// Test: parse CLUSTER NODES with replicas.
    ///
    /// Replicas have no slot ownership.
    #[test]
    fn test_parse_cluster_nodes_with_replicas() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 master - 0 1700000000000 1 connected 0-16383\n\
bbb000bbb000bbb000bbb000bbb000bbb000 192.168.1.11:6379@16379 slave aaa000aaa000aaa000aaa000aaa000aaa000 0 1700000000000 1 connected\n\
";

        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 2);

        // Master owns all slots
        assert_eq!(
            map.node_for_slot(0),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(16383),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );

        // Replica has no slot ownership — slot 0 is owned by the master, not the replica
        assert_eq!(
            map.node_for_slot(0),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        let replica_info =
            map.node_info(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"));
        assert!(replica_info.is_some());
        assert!(replica_info.unwrap().slots.is_none());
    }

    /// Test: parse node with fail/pfail flags → NodeState::Down.
    #[test]
    fn test_parse_cluster_nodes_fail_flag() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 fail,master - 0 1700000000000 1 connected 0-5460\n\
bbb000bbb000bbb000bbb000bbb000bbb000 192.168.1.11:6379@16379 pfail,master - 0 1700000000000 2 connected 5461-16383\n";

        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 2);

        // Both fail/pfail → both are Down
        for node in map.nodes() {
            assert_eq!(node.state, NodeState::Down);
        }

        // All 16384 slots assigned
        assert!(map.is_complete());
    }

    /// Test: parse CLUSTER SLOTS with 3 masters.
    #[test]
    fn test_parse_cluster_slots_3_masters() {
        let value = RedisValue::Array(vec![
            RedisValue::Array(vec![
                RedisValue::Integer(0),
                RedisValue::Integer(5460),
                RedisValue::Array(vec![
                    RedisValue::BulkString(b"192.168.1.10".to_vec()),
                    RedisValue::Integer(6379),
                    RedisValue::BulkString(
                        b"aaa000aaa000aaa000aaa000aaa000aaa000".to_vec(),
                    ),
                ]),
            ]),
            RedisValue::Array(vec![
                RedisValue::Integer(5461),
                RedisValue::Integer(10922),
                RedisValue::Array(vec![
                    RedisValue::BulkString(b"192.168.1.11".to_vec()),
                    RedisValue::Integer(6379),
                    RedisValue::BulkString(
                        b"bbb000bbb000bbb000bbb000bbb000bbb000".to_vec(),
                    ),
                ]),
            ]),
            RedisValue::Array(vec![
                RedisValue::Integer(10923),
                RedisValue::Integer(16383),
                RedisValue::Array(vec![
                    RedisValue::BulkString(b"192.168.1.12".to_vec()),
                    RedisValue::Integer(6379),
                    RedisValue::BulkString(
                        b"ccc000ccc000ccc000ccc000ccc000ccc000".to_vec(),
                    ),
                ]),
            ]),
        ]);

        let map = parse_cluster_slots(&value).unwrap();
        assert_eq!(map.len(), 3);
        assert!(map.is_complete());

        // Verify slot boundaries
        assert_eq!(
            map.node_for_slot(5460),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(5461),
            Some(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"))
        );
        assert_eq!(
            map.node_for_slot(10923),
            Some(mknodeid("ccc000ccc000ccc000ccc000ccc000ccc000"))
        );
    }

    /// Test: parse CLUSTER SLOTS with replicas.
    #[test]
    fn test_parse_cluster_slots_with_replicas() {
        let value = RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::Integer(0),
            RedisValue::Integer(5460),
            RedisValue::Array(vec![
                RedisValue::BulkString(b"192.168.1.10".to_vec()),
                RedisValue::Integer(6379),
                RedisValue::BulkString(
                    b"aaa000aaa000aaa000aaa000aaa000aaa000".to_vec(),
                ),
            ]),
            RedisValue::Array(vec![
                RedisValue::BulkString(b"192.168.1.11".to_vec()),
                RedisValue::Integer(6380),
                RedisValue::BulkString(
                    b"bbb000bbb000bbb000bbb000bbb000bbb000".to_vec(),
                ),
            ]),
        ])]);

        let map = parse_cluster_slots(&value).unwrap();
        assert_eq!(map.len(), 2); // 1 master + 1 replica

        // Verify master owns slots
        assert_eq!(
            map.node_for_slot(0),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(5460),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );

        // Replica has no slots
        let replica_info =
            map.node_info(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"));
        assert!(replica_info.is_some());
        assert_eq!(replica_info.unwrap().role, NodeRole::Replica);
        assert!(replica_info.unwrap().slots.is_none());
    }

    /// Test: empty CLUSTER NODES response.
    #[test]
    fn test_parse_empty_cluster_nodes() {
        let map = parse_cluster_nodes("").unwrap();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    /// Test: invalid CLUSTER NODES response (too few fields).
    #[test]
    fn test_parse_invalid_cluster_nodes() {
        let result = parse_cluster_nodes("just-some-text");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RedisError::Parse(_)));
    }

    /// Test: CLUSTERDOWN detection with slot number.
    #[test]
    fn test_parse_clusterdown_with_slot() {
        let value = RedisValue::Error(
            "-CLUSTERDOWN The cluster is down - hash slot 3999 not owned".into(),
        );
        let slot = parse_clusterdown(&value);
        assert_eq!(slot, Some(3999));
    }

    /// Test: CLUSTERDOWN detection without slot number.
    #[test]
    fn test_parse_clusterdown_without_slot() {
        let value = RedisValue::Error("-CLUSTERDOWN Cluster is shut down".into());
        let slot = parse_clusterdown(&value);
        assert_eq!(slot, None);
    }

    /// Test: non-CLUSTERDOWN response.
    #[test]
    fn test_parse_non_clusterdown() {
        let value = RedisValue::Error("-ERR unknown command FOO".into());
        let slot = parse_clusterdown(&value);
        assert_eq!(slot, None);
    }

    /// Test: CLUSTERDOWN detection on non-error value.
    #[test]
    fn test_parse_clusterdown_not_error() {
        let value = RedisValue::SimpleString("OK".into());
        let slot = parse_clusterdown(&value);
        assert_eq!(slot, None);
    }

    /// Test: CLUSTER SLOTS response is not an array.
    #[test]
    fn test_parse_cluster_slots_not_array() {
        let value = RedisValue::SimpleString("OK".into());
        let result = parse_cluster_slots(&value);
        assert!(result.is_err());
    }

    /// Test: CLUSTER SLOTS entry has too few fields.
    #[test]
    fn test_parse_cluster_slots_bad_entry() {
        let value =
            RedisValue::Array(vec![RedisValue::Array(vec![RedisValue::Integer(0)])]);
        let result = parse_cluster_slots(&value);
        assert!(result.is_err());
    }

    /// Test: CLUSTER SLOTS with empty node-id (uses placeholder).
    #[test]
    fn test_parse_cluster_slots_empty_node_id() {
        let value = RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::Integer(0),
            RedisValue::Integer(5460),
            RedisValue::Array(vec![
                RedisValue::BulkString(b"192.168.1.10".to_vec()),
                RedisValue::Integer(6379),
                RedisValue::BulkString(b"".to_vec()), // empty node-id
            ]),
        ])]);

        let map = parse_cluster_slots(&value).unwrap();
        assert_eq!(map.len(), 1);

        // Should use the placeholder node ID
        let node = map.node_for_slot(0);
        assert!(node.is_some());
    }

    /// Test: parse empty line (should skip gracefully).
    #[test]
    fn test_parse_cluster_nodes_empty_lines() {
        let text = "\n\naaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 master - 0 1700000000000 1 connected 0-16383\n\n\n";
        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.is_complete());
    }

    /// Test: single master owns all 16384 slots.
    #[test]
    fn test_parse_single_master_all_slots() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 myself,master - 0 1700000000000 1 connected 0-16383\n";

        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.is_complete());

        // Verify boundaries
        assert_eq!(
            map.node_for_slot(0),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(8192),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
        assert_eq!(
            map.node_for_slot(16383),
            Some(mknodeid("aaa000aaa000aaa000aaa000aaa000aaa000"))
        );
    }

    /// Test: link state disconnected.
    #[test]
    fn test_parse_disconnected_link_state() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 master,disconnected - 0 1700000000000 1 disconnected 0-5460\n";

        let result = parse_cluster_nodes(text);
        // The current code checks for "connected"/"disconnected" but only
        // accepts them as flags_str, not as link_state. The link_state is
        // at index 7, which is "disconnected".
        // This should parse successfully.
        assert!(result.is_ok());
    }

    /// Test: node with slave flag and no slots.
    #[test]
    fn test_parse_slave_no_slots() {
        let text = "\
bbb000bbb000bbb000bbb000bbb000bbb000 192.168.1.11:6379@16379 slave aaa000aaa000aaa000aaa000aaa000aaa000 0 1700000000000 1 connected\n";

        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 1);

        let node = map.node_info(mknodeid("bbb000bbb000bbb000bbb000bbb000bbb000"));
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.role, NodeRole::Replica);
        assert!(node.slots.is_none());
    }

    /// Test: my self flag.
    #[test]
    fn test_parse_myself_flag() {
        let text = "\
aaa000aaa000aaa000aaa000aaa000aaa000 192.168.1.10:6379@16379 myself,master - 0 1700000000000 1 connected 0-5460\n";

        let map = parse_cluster_nodes(text).unwrap();
        assert_eq!(map.len(), 1);
    }

    /// Test: replica with empty/missing node-id.
    #[test]
    fn test_parse_replica_no_node_id() {
        let value = RedisValue::Array(vec![RedisValue::Array(vec![
            RedisValue::Integer(0),
            RedisValue::Integer(5460),
            RedisValue::Array(vec![
                RedisValue::BulkString(b"192.168.1.10".to_vec()),
                RedisValue::Integer(6379),
            ]),
            RedisValue::Array(vec![
                RedisValue::BulkString(b"192.168.1.11".to_vec()),
                RedisValue::Integer(6380),
            ]),
        ])]);

        let map = parse_cluster_slots(&value).unwrap();
        assert_eq!(map.len(), 2);
    }
}
