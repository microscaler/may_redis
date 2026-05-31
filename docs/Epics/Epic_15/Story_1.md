# Story 15.1 — ClusterClient Connect + Basic Routing

**Objective:** Create `RedisClusterClient` that connects to seed nodes, builds a SlotMap, and routes single-key commands to the correct node via hash slot.

**Epic:** 15 — Redis Cluster Support

**Dependencies:** Epic 0 (single-crate module structure), Epic 14 (connection pattern — `Connection::connect`, `send`, `execute`)

**Source docs:** `docs/PRD-redis-cluster.md` §2.1, §3.2, §5.2, §5.3, §10.2

## Functional Requirements

- **FR-1: ClusterClient::connect** — Create a cluster client from seed node addresses
  - Method: `pub async fn connect(seeds: &[&str]) -> Result<Self, RedisError>`
  - For each seed: attempt TCP connect, send `CLUSTER NODES`, parse response to build SlotMap
  - Stop on first seed that returns valid slot data
  - Store seed nodes for periodic refresh
- **FR-2: Basic slot-based routing** — Route single-key commands to correct node
  - Method: `pub async fn execute<T: FromRedisValue>(&self, cmd: CommandBuilder) -> Result<T, RedisError>`
  - Extract key from CommandBuilder → compute CRC16 slot → lookup slot_map → get Connection → send command
- **FR-3: Node selection by slot** — Look up Connection for a given slot
  - Method: `fn connection_for_slot(&self, slot: u16) -> Result<&Connection, RedisError>`
  - Returns error if slot is unassigned (unknown slot)
- **FR-4: Single-key command support** — Only commands with one key are routed
  - `GET key`, `SET key value`, `DEL key`, `EXISTS key`, `TTL key`, `EXPIRE key seconds`
  - `INCR key` (key is argument 1)

## Non-Functional Requirements

- Feature-gate all cluster code behind `#[cfg(feature = "cluster")]`
- No new dependencies in Cargo.toml (reuses `bytes`, `may`, `socket2`, `log`)
- Follows existing `Connection` API patterns (epoll loop, mpsc, spsc)
- Every public method has error documentation

## Code Anchors

- `src/cluster/mod.rs` — add `pub mod cluster_client;`, add `pub use cluster_client::RedisClusterClient;`
- `src/cluster/cluster_client.rs` — **NEW** — `RedisClusterClient`, `ClusterInner`, routing logic
- `src/cluster/crc16.rs` — existing `compute_slot()` (read-only reference)
- `src/cluster/slot_map.rs` — existing `SlotMap`, `NodeInfo`, `NodeId` (read-only reference)
- `src/client/client.rs` — existing `RedisClient` — pattern to mirror for `execute()`
- `src/connection/connection.rs` — existing `Connection::send()`, `Connection::connect()`
- `src/lib.rs` — add `#[cfg(feature = "cluster")] pub use cluster::RedisClusterClient;`
- `Cargo.toml` — add `cluster = []` to `[features]`

## Structs

```rust
/// A seed node for initial topology discovery.
#[derive(Debug, Clone)]
pub struct SeedNode {
    pub host: String,
    pub port: u16,
}

/// Policy for topology refresh.
#[derive(Debug, Clone)]
pub enum RefreshPolicy {
    /// Refresh every N seconds.
    Periodic(Duration),
    /// Never refresh automatically.
    Manual,
    /// Refresh on cluster errors and periodically.
    OnErrorAndPeriodic(Duration),
}

/// Internal cluster state shared across coroutines.
pub(crate) struct ClusterInner {
    /// Slot → NodeId mapping.
    pub slot_map: SlotMap,
    /// NodeId → Connection.
    pub connections: HashMap<NodeId, Connection>,
    /// Seed nodes for topology discovery.
    pub seed_nodes: Vec<SeedNode>,
    /// Refresh policy.
    pub refresh_policy: RefreshPolicy,
}

/// Main entry point for Redis Cluster operations.
///
/// `RedisClusterClient` wraps `Arc<ClusterInner>` so multiple coroutines
/// can share the same cluster connections.
#[derive(Clone)]
pub struct RedisClusterClient {
    inner: Arc<ClusterInner>,
}
```

## Tasks

- [ ] Add `cluster = []` feature flag to Cargo.toml
- [ ] Add `pub mod cluster_client;` to `src/cluster/mod.rs`
- [ ] Implement `SeedNode`, `RefreshPolicy` structs
- [ ] Implement `ClusterInner` with `SlotMap`, `HashMap<NodeId, Connection>`, seed nodes
- [ ] Implement `RedisClusterClient::connect(seeds: &[&str])` — connect to each seed, send `CLUSTER NODES`, build slot map
- [ ] Implement `RedisClusterClient::execute<T: FromRedisValue>(&self, cmd: CommandBuilder) -> Result<T, RedisError>` — extract key, compute slot, lookup connection, send, receive
- [ ] Implement `ClusterInner::connection_for_slot(&self, slot: u16)` — return connection for slot
- [ ] Wire `RedisClusterClient` re-export in `src/lib.rs` behind `#[cfg(feature = "cluster")]`
- [ ] Add unit tests: CRC16 slot lookup for known keys, slot map node resolution
- [ ] Add integration test: connect to 3-node cluster, route GET/SET to correct nodes
- [ ] Run `cargo build --features cluster`
- [ ] Run `cargo test --lib --features cluster -- --test-threads=1`
- [ ] Run `cargo clippy --lib --features cluster -- -D warnings`

## Verification

- `cargo build --features cluster` — compiles with cluster feature
- `cargo build` — compiles without cluster feature (no cluster types present)
- `cargo test --lib --features cluster -- cluster::cluster_client::tests::test_` — unit tests pass
- `cargo clippy --lib --features cluster -- -D warnings` — zero warnings
