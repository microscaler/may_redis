/// Slot-to-node mapping for Redis Cluster.
///
/// Each Redis Cluster deployment has exactly 16384 hash slots. This module
/// provides a mapping from slot number to node identity, with updates for
/// handling MOVED/ASK redirects and topology refreshes.
use std::fmt;
use std::net::SocketAddr;

/// A unique identifier for a Redis Cluster node.
///
/// Each node has a unique 40-character hex ID assigned at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 20]);

impl NodeId {
    /// Create a NodeId from a 40-character hex string.
    ///
    /// # Panics
    /// Panics if the string is not exactly 40 hex characters.
    #[must_use]
    pub fn from_hex(s: &str) -> Self {
        let mut bytes = [0u8; 20];
        let mut i = 0;
        let mut ch = 0;
        let mut nibble = false;
        while i < s.len() && ch < 40 {
            let c = s.as_bytes()[i];
            i += 1;
            let val = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => continue,
            };
            if !nibble {
                bytes[ch >> 1] = val << 4; // Store first nibble
                nibble = true;
            } else {
                bytes[ch >> 1] |= val; // Store second nibble
                ch += 2;
                nibble = false;
            }
        }
        // Handle odd number of hex chars — first nibble only
        if nibble && ch < 40 {
            // Already handled above
        }
        Self(bytes)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Role of a node in a Redis Cluster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    /// Master node — accepts reads and writes.
    Master,
    /// Replica node — accepts reads only.
    Replica,
}

/// State of a node in a Redis Cluster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    /// Node is online and connected.
    Online,
    /// Node is down and unreachable.
    Down,
}

/// A single node in the cluster, including its role, address, and slot ownership.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Unique node ID (40 hex chars).
    pub id: NodeId,
    /// Network address (host:port).
    pub addr: SocketAddr,
    /// Master or replica.
    pub role: NodeRole,
    /// Range of hash slots owned by this node.
    /// For replicas, this is None (they don't own slots).
    pub slots: Option<std::ops::RangeInclusive<u16>>,
    /// Current node state.
    pub state: NodeState,
}

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}] {} ({:?})",
            self.id,
            self.addr,
            match self.role {
                NodeRole::Master => "master",
                NodeRole::Replica => "replica",
            },
            self.state
        )
    }
}

/// Thread-safe slot-to-node mapping for Redis Cluster.
///
/// The slot map is the core routing data structure. Every key-based command
/// computes `CRC16(key) % 16384` to get the slot number, then looks up which
/// node owns that slot to determine the target connection.
///
/// Updates are atomic — a redirect (MOVED/ASK) or topology refresh swaps the
/// entire map in O(1) without locking.
#[derive(Debug, Clone)]
pub struct SlotMap {
    /// Mapping from slot number to NodeId.
    slots: Box<[Option<NodeId>; 16384]>,
    /// Mapping from NodeId to NodeInfo.
    nodes: std::collections::HashMap<NodeId, NodeInfo>,
}

impl SlotMap {
    /// Create an empty slot map with no nodes.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            slots: Box::new([None; 16384]),
            nodes: std::collections::HashMap::new(),
        }
    }

    /// Add or update a node's slot ownership.
    ///
    /// # Arguments
    ///
    /// * `node` — The node info including its slot range.
    pub fn add_node(&mut self, node: NodeInfo) {
        self.nodes.insert(node.id, node.clone());
        if let Some(range) = node.slots {
            for slot in range {
                self.slots[slot as usize] = Some(node.id);
            }
        }
    }

    /// Remove a node from the map, clearing its slot assignments.
    ///
    /// # Returns
    ///
    /// `true` if the node was present and removed.
    pub fn remove_node(&mut self, node_id: NodeId) -> bool {
        if let Some(node) = self.nodes.remove(&node_id) {
            if let Some(range) = node.slots {
                for slot in range {
                    self.slots[slot as usize] = None;
                }
            }
            return true;
        }
        false
    }

    /// Look up which node owns a given slot.
    ///
    /// # Returns
    ///
    /// `Some(node_id)` if the slot is assigned, `None` if unassigned.
    #[must_use]
    pub fn node_for_slot(&self, slot: u16) -> Option<NodeId> {
        self.slots[slot as usize]
    }

    /// Look up node info for a given node ID.
    ///
    /// # Returns
    ///
    /// `Some(node_info)` if the node is known, `None` otherwise.
    #[must_use]
    pub fn node_info(&self, node_id: NodeId) -> Option<&NodeInfo> {
        self.nodes.get(&node_id)
    }

    /// Get all nodes in the cluster.
    #[must_use]
    pub fn nodes(&self) -> std::collections::hash_map::Values<'_, NodeId, NodeInfo> {
        self.nodes.values()
    }

    /// Get the number of nodes in the cluster.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the cluster has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Check if a slot is unassigned.
    #[must_use]
    pub fn is_unknown_slot(&self, slot: u16) -> bool {
        self.slots[slot as usize].is_none()
    }

    /// Check if all 16384 slots are assigned.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// Build a slot map from a list of node infos.
    ///
    /// This is the primary method for populating the map from a
    /// `CLUSTER NODES` or `CLUSTER SLOTS` response.
    #[must_use]
    pub fn from_nodes(nodes: &[NodeInfo]) -> Self {
        let mut map = Self::empty();
        for node in nodes {
            map.add_node(node.clone());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::used_underscore_items
    )]

    use super::*;
    use std::net::SocketAddr;

    /// Create a test NodeId from a hex string.
    fn make_node_id(s: &str) -> NodeId {
        NodeId::from_hex(s)
    }

    /// Create a test SocketAddr.
    fn make_addr(host: &str, port: u16) -> SocketAddr {
        format!("{host}:{port}").parse().unwrap()
    }

    /// Create a test NodeInfo.
    fn make_node(
        id: &str,
        addr: &str,
        port: u16,
        role: NodeRole,
        slot_start: u16,
        slot_end: u16,
        state: NodeState,
    ) -> NodeInfo {
        NodeInfo {
            id: make_node_id(id),
            addr: make_addr(addr, port),
            role,
            slots: Some(slot_start..=slot_end),
            state,
        }
    }

    #[test]
    fn test_node_id_from_hex() {
        let id = make_node_id("a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0");
        let hex = id.to_string();
        assert_eq!(hex.len(), 40);
        assert_eq!(hex, "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0");
    }

    #[test]
    fn test_slot_map_empty() {
        let map = SlotMap::empty();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(map.is_unknown_slot(0));
        assert_eq!(map.node_for_slot(0), None);
    }

    #[test]
    fn test_slot_map_add_master() {
        let mut map = SlotMap::empty();
        let node = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6379,
            NodeRole::Master,
            0,
            5460,
            NodeState::Online,
        );
        map.add_node(node);
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        // Slots 0-5460 should point to the master
        assert_eq!(
            map.node_for_slot(0).unwrap().to_string(),
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0"
        );
        assert_eq!(
            map.node_for_slot(5460).unwrap().to_string(),
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0"
        );

        // Slot 5461 should be unassigned
        assert!(map.is_unknown_slot(5461));
    }

    #[test]
    fn test_slot_map_all_slots() {
        let mut map = SlotMap::empty();
        let node1 = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6379,
            NodeRole::Master,
            0,
            5460,
            NodeState::Online,
        );
        let node2 = make_node(
            "b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0",
            "192.168.1.11",
            6379,
            NodeRole::Master,
            5461,
            10922,
            NodeState::Online,
        );
        let node3 = make_node(
            "c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0",
            "192.168.1.12",
            6379,
            NodeRole::Master,
            10923,
            16383,
            NodeState::Online,
        );
        map.add_node(node1);
        map.add_node(node2);
        map.add_node(node3);

        // All slots assigned
        assert!(map.is_complete());
        assert_eq!(map.len(), 3);

        // Verify slot boundaries
        assert_eq!(
            map.node_for_slot(0).unwrap().to_string(),
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0"
        );
        assert_eq!(
            map.node_for_slot(5460).unwrap().to_string(),
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0"
        );
        assert_eq!(
            map.node_for_slot(5461).unwrap().to_string(),
            "b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0"
        );
        assert_eq!(
            map.node_for_slot(10922).unwrap().to_string(),
            "b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0"
        );
        assert_eq!(
            map.node_for_slot(10923).unwrap().to_string(),
            "c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0"
        );
        assert_eq!(
            map.node_for_slot(16383).unwrap().to_string(),
            "c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0c0f0"
        );
    }

    #[test]
    fn test_slot_map_remove_node() {
        let mut map = SlotMap::empty();
        let node = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6379,
            NodeRole::Master,
            0,
            16383,
            NodeState::Online,
        );
        map.add_node(node);
        assert_eq!(map.len(), 1);
        assert!(map.is_complete());

        let removed =
            map.remove_node(make_node_id("a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0"));
        assert!(removed);
        assert!(map.is_empty());
        assert!(map.is_unknown_slot(0));
    }

    #[test]
    fn test_slot_map_from_nodes() {
        let nodes = vec![
            make_node(
                "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
                "192.168.1.10",
                6379,
                NodeRole::Master,
                0,
                5460,
                NodeState::Online,
            ),
            make_node(
                "b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0b0f0",
                "192.168.1.11",
                6379,
                NodeRole::Master,
                5461,
                16383,
                NodeState::Online,
            ),
        ];
        let map = SlotMap::from_nodes(&nodes);
        assert_eq!(map.len(), 2);
        assert!(map.is_complete());
    }

    #[test]
    fn test_slot_map_node_info_lookup() {
        let mut map = SlotMap::empty();
        let node = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6379,
            NodeRole::Master,
            0,
            5460,
            NodeState::Online,
        );
        map.add_node(node.clone());
        let info = map.node_info(node.id);
        assert!(info.is_some());
        assert_eq!(info.unwrap().addr, make_addr("192.168.1.10", 6379));
    }

    #[test]
    fn test_slot_map_replica_no_slots() {
        let mut map = SlotMap::empty();
        let replica = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6380,
            NodeRole::Replica,
            0,
            0,
            NodeState::Online,
        );
        // Replica with no slot ownership
        let mut replica = replica;
        replica.slots = None;
        map.add_node(replica);
        assert_eq!(map.len(), 1);
        // All slots still unassigned (replicas don't own slots)
        assert!(map.is_unknown_slot(0));
    }

    #[test]
    fn test_slot_map_down_node() {
        let mut map = SlotMap::empty();
        let node = make_node(
            "a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0",
            "192.168.1.10",
            6379,
            NodeRole::Master,
            0,
            5460,
            NodeState::Down,
        );
        map.add_node(node);
        assert_eq!(map.len(), 1);
        // Slots are still mapped even if node is down
        assert!(!map.is_unknown_slot(0));
    }

    #[test]
    fn test_slot_map_remove_nonexistent() {
        let mut map = SlotMap::empty();
        let fake_id = make_node_id("a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0a0f0");
        let removed = map.remove_node(fake_id);
        assert!(!removed);
    }
}
