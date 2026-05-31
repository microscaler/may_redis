/// Main cluster client with multi-node connection management and slot-based
/// command routing.
pub mod cluster_client;
/// CRC16-ANSI hash function and Redis Cluster slot computation.
///
/// Redis Cluster uses `CRC16(key) mod 16384` to map keys to 16384 hash slots.
/// This module provides the hash function (`crc16`) and slot computation
/// (`compute_slot`) as pure, no-std-compatible functions.
pub mod crc16;
/// Slot-to-node mapping for Redis Cluster.
///
/// Each Redis Cluster deployment has exactly 16384 hash slots. This module
/// provides a mapping from slot number to node identity, with updates for
/// handling MOVED/ASK redirects and topology refreshes.
pub mod slot_map;
/// CLUSTER NODES and CLUSTER SLOTS response parsing.
pub mod topology;

pub use cluster_client::{RefreshPolicy, SeedNode};
pub use crc16::{compute_slot, crc16};
pub use slot_map::{NodeId, NodeInfo, NodeRole, NodeState, SlotMap};
