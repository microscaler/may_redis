# Story 15.3 — Topology Discovery

**Objective:** Parse `CLUSTER NODES` and `CLUSTER SLOTS` responses to discover and maintain the cluster topology, with support for periodic and on-demand refresh.

**Epic:** 15 — Redis Cluster Support

**Dependencies:** Story 15.1 (ClusterClient connect), Story 15.2 (Redirect handling)

**Source docs:** `docs/PRD-redis-cluster.md` §2.3, §2.4, §5.5, §10.4

## Functional Requirements

- **FR-1: CLUSTER NODES parsing** — Parse `CLUSTER NODES` output into `NodeInfo` list
  - Method: `fn parse_cluster_nodes(response: &RedisValue) -> Result<Vec<NodeInfo>, RedisError>`
  - Format: `<node-id> <addr>:<port>@<bus-port> <flags> <master-id> <ping-sent> <pong-received> <config-epoch> <link-state> <slot> ... <slot>`
  - Flags: `master`, `slave`, `fail`, `pfail`, `fail?` (parsed into `NodeState`)
  - Parse slot ranges like `0-5460` into `RangeInclusive<u16>`
- **FR-2: CLUSTER SLOTS parsing** — Parse `CLUSTER SLOTS` output into `NodeInfo` list
  - Method: `fn parse_cluster_slots(response: &RedisValue) -> Result<Vec<NodeInfo>, RedisError>`
  - Format: array of arrays: `[[start_slot, end_slot, ip, port, ...], ...]`
  - Include replica information from nested arrays
- **FR-3: Topology refresh from seed nodes** — Query all known nodes
  - Method: `async fn refresh_topology(&self) -> Result<(), RedisError>`
  - Send `CLUSTER NODES` to each known node (round-robin)
  - Update slot map with parsed results
  - Use first successful response
- **FR-4: On-demand refresh triggers** — Refresh on cluster errors
  - On `CLUSTERDOWN` response: trigger full refresh
  - On connection error to any node: trigger refresh from other nodes
  - On `unknown slot` error: trigger refresh (slot not in map)
- **FR-5: Periodic refresh support** — Background refresh task
  - Method: `async fn start_periodic_refresh(&self, interval: Duration)`
  - Spawn a `may::go!` coroutine that calls `refresh_topology()` every interval
  - Configurable via `RefreshPolicy::Periodic(Duration)` or `OnErrorAndPeriodic(Duration)`

## Non-Functional Requirements

- Every parser has unit tests with exact `CLUSTER NODES` and `CLUSTER SLOTS` RESP formats
- Parser errors are descriptive (include the raw response in error message)
- Refresh is idempotent: calling it multiple times produces the same result
- No blocking sleep — use `may::timer::sleep` for periodic refresh

## Code Anchors

- `src/cluster/topology.rs` — **NEW** — `parse_cluster_nodes()`, `parse_cluster_slots()`, `refresh_topology()`
- `src/cluster/mod.rs` — add `pub mod topology;`, add re-exports
- `src/cluster/cluster_client.rs` — existing `ClusterInner` — add `refresh_topology()` method, call on redirect/CLUSTERDOWN
- `src/cluster/slot_map.rs` — existing `SlotMap` — add_node, remove_node used during refresh
- `src/core/value.rs` — existing `RedisValue` — for parsing RESP arrays
- `src/lib.rs` — no changes needed

## Structs

```rust
/// Information about a cluster node's connection bus.
#[derive(Debug, Clone)]
pub struct NodeBusInfo {
    pub bus_port: u16,
    pub flags: Vec<NodeFlag>,
    pub master_id: Option<NodeId>,
    pub ping_sent: u64,
    pub pong_received: u64,
    pub config_epoch: u64,
    pub link_state: NodeLinkState,
}

/// Parsed flags from CLUSTER NODES output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeFlag {
    Myself,
    Master,
    Slave,
    Fail,
    Pfail,
    FailMarked,  // fail?
}

/// Link state from CLUSTER NODES output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeLinkState {
    Connected,
    Disconnected,
}
```

## Tasks

- [x] Add `NodeBusInfo`, `NodeFlag`, `NodeLinkState` structs
- [x] Implement `parse_cluster_nodes()` — parse RESP bulk string into `Vec<NodeInfo>`
- [x] Implement `parse_cluster_slots()` — parse RESP nested array into `Vec<NodeInfo>`
- [ ] Implement `ClusterInner::refresh_topology()` — send CLUSTER NODES to each node
- [ ] Implement periodic refresh: `ClusterInner::start_periodic_refresh()` with `may::timer::sleep`
- [ ] Wire refresh triggers in `execute()`: on CLUSTERDOWN, on connection error, on unknown slot
- [x] Wire `topology` module in `src/cluster/mod.rs`
- [x] Unit tests: parse 3-node CLUSTER NODES response (standard split)
- [x] Unit tests: parse CLUSTER SLOTS response with replicas
- [x] Unit tests: parse node with fail/pfail flags
- [x] Unit tests: parse empty/invalid CLUSTER NODES response
- [ ] Integration test: discover 3-node cluster from single seed
- [ ] Integration test: refresh topology after node removal
- [ ] Run `cargo build --features cluster`
- [ ] Run `cargo test --lib --features cluster -- --test-threads=1`
- [ ] Run `cargo clippy --lib --features cluster -- -D warnings`

**Unit test results: 365 passed, 0 failed, 113 ignored. All topology parsing tests pass.**

## Verification

### Unit Tests

- `test_parse_cluster_nodes_3_masters` — 3-node standard cluster → 3 NodeInfo, all slots assigned
- `test_parse_cluster_nodes_with_replicas` — includes replica nodes with no slot ownership
- `test_parse_cluster_nodes_fail_flag` — node with `fail` flag → `NodeState::Down`
- `test_parse_cluster_slots_3_masters` — CLUSTER SLOTS output → 3 NodeInfo, all slots assigned
- `test_parse_cluster_slots_with_replicas` — replica nodes parsed, no slots
- `test_parse_empty_cluster_nodes` — empty string → `RedisError::Parse`
- `test_parse_invalid_cluster_nodes` — malformed data → `RedisError::Parse`

### Integration Tests

- `test_discover_cluster_from_seed` — connect to one seed, discover all 3 nodes
- `test_refresh_after_node_removal` — remove a node from cluster, refresh updates slot map
- `test_refresh_after_slot_migration` — resharding in progress, refresh detects new slot assignment

### Build & Lint

- `cargo build --features cluster` — compiles
- `cargo clippy --lib --features cluster -- -D warnings` — zero warnings
