/// CLUSTER NODES and CLUSTER SLOTS response parsing.
///
/// Parses the textual output of `CLUSTER NODES` and the array output of
/// `CLUSTER SLOTS` into `SlotMap` / `NodeInfo` data structures.
use std::net::SocketAddr;

use crate::cluster::slot_map::{NodeInfo, NodeRole, NodeState, SlotMap};
use crate::core::RedisError;

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
pub fn parse_cluster_nodes(text: &str) -> Result<SlotMap, RedisError> {
    let mut map = SlotMap::empty();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            return Err(RedisError::Parse(format!(
                "malformed CLUSTER NODES entry (too few fields): {line}"
            )));
        }

        let node_id_str = parts[0];
        let addr_str = parts[1];
        let flags_str = parts[2];

        // Parse address: host:port@bus-port
        let (host_port, _) = addr_str.rsplit_once('@').ok_or_else(|| {
            RedisError::Parse(format!("malformed address: {addr_str}"))
        })?;
        let (host, port_str) = host_port.rsplit_once(':').ok_or_else(|| {
            RedisError::Parse(format!("malformed host:port: {host_port}"))
        })?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| RedisError::Parse(format!("invalid port: {port_str}")))?;
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

        // Parse slot ranges (remaining fields after link-state).
        let slot_ranges = &parts[9..];
        let mut slots: Vec<u16> = Vec::new();
        for slot_spec in slot_ranges {
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
            id: crate::cluster::slot_map::NodeId::from_hex(node_id_str),
            addr,
            role,
            state,
            slots: if slots.is_empty() {
                None
            } else {
                let start = *slots.first().unwrap();
                let end = *slots.last().unwrap();
                Some(start..=end)
            },
        };

        map.add_node(node);
    }

    Ok(map)
}
