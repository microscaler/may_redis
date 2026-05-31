# Story 15.5 — Pipeline Across Nodes

**Objective:** Support Redis pipelines that span multiple cluster nodes — each command goes to the correct node, responses are collected per-node, and results are aggregated.

**Epic:** 15 — Redis Cluster Support

**Dependencies:** Story 15.1 (ClusterClient), Story 15.2 (Redirect handling), Story 15.3 (Topology discovery), Story 15.4 (Multi-key fan-out)

**Source docs:** `docs/PRD-redis-cluster.md` §5.6, §10.6

## Functional Requirements

- **FR-1: ClusterPipeline struct** — Pipeline that spans multiple nodes
  - Method: `pub fn cluster_pipeline(&self) -> ClusterPipeline<'_>`
  - Similar to `RedisClient::pipeline()` but routes per-command to correct node
- **FR-2: Command routing in pipeline** — Each `add()` command goes to its target node
  - Extract key from each command → compute slot → get connection
  - Group commands by target node (batch per-node)
  - Each node's batch is sent as a single pipeline
- **FR-3: Response collection and ordering** — Maintain command ordering across nodes
  - Each command gets a sequence number on `add()`
  - Responses are collected per-node in order
  - Final result is ordered by original sequence number
- **FR-4: Pipeline execute** — Flush all per-node pipelines and collect results
  - Method: `pub fn execute(&mut self) -> Result<Vec<RedisValue>, RedisError>`
  - Send all per-node pipelines concurrently (one `go!` per node)
  - Collect responses via spsc channels
  - Reorder by sequence number
- **FR-5: Typed pipeline execution** — Decode typed results via `FromPipelineResponse`
  - Method: `pub fn execute_typed<T: FromPipelineResponse>(&mut self) -> Result<T, RedisError>`

## Non-Functional Requirements

- Pipelines across nodes use may coroutines for concurrent per-node execution
- Timeout applies to the entire pipeline operation
- If ANY node's pipeline fails, return the error (fail-fast)
- Same redirect handling as single commands: MOVED/ASK → update slot map, retry

## Code Anchors

- `src/cluster/pipeline.rs` — **NEW** — `ClusterPipeline`, routing, concurrent execute
- `src/cluster/mod.rs` — add `pub mod pipeline;`, add re-exports
- `src/cluster/cluster_client.rs` — existing `RedisClusterClient` — add `cluster_pipeline()` method
- `src/client/pipeline.rs` — existing `Pipeline` — pattern to follow for `ClusterPipeline`
- `src/client/pipeline_response.rs` — existing `FromPipelineResponse` — for typed results

## Structs

```rust
/// A command queued in a cluster pipeline.
#[derive(Debug)]
pub struct PipelineCommand {
    /// Original sequence number for ordering.
    pub seq: usize,
    /// The RESP-encoded command bytes.
    pub data: Vec<u8>,
    /// The channel sender for the response.
    pub sender: may::sync::spsc::Sender<RedisValue>,
}

/// Batch commands grouped by target node.
#[derive(Debug)]
pub struct NodeBatch {
    /// Target connection for this node.
    pub connection: Connection,
    /// Commands targeting this node, in order.
    pub commands: Vec<PipelineCommand>,
}

/// Pipeline that spans multiple cluster nodes.
pub struct ClusterPipeline<'a> {
    inner: Arc<ClusterInner>,
    commands: Vec<PipelineCommand>,
    next_seq: usize,
}
```

## Tasks

- [ ] Add `PipelineCommand`, `NodeBatch`, `ClusterPipeline` structs
- [ ] Implement `ClusterClient::cluster_pipeline()` — creates empty `ClusterPipeline`
- [ ] Implement `ClusterPipeline::add(cmd: CommandBuilder)` — routes to target node, stores batch
- [ ] Implement `ClusterPipeline::execute()` — group by node, send per-node pipeline, collect, reorder
- [ ] Implement `ClusterPipeline::execute_typed<T: FromPipelineResponse>()` — decode typed results
- [ ] Handle redirects in pipeline: if a node returns MOVED/ASK, update slot map, retry that node's batch
- [ ] Wire `pipeline` module in `src/cluster/mod.rs`
- [ ] Unit tests: route 3 commands to 2 nodes, verify batch grouping
- [ ] Unit tests: response ordering — commands to different nodes returned in original order
- [ ] Integration test: pipeline with GET/SET across 3 nodes → all results in order
- [ ] Integration test: pipeline with redirect — one node returns MOVED, retries on new node
- [ ] Run `cargo build --features cluster`
- [ ] Run `cargo test --lib --features cluster -- --test-threads=1`
- [ ] Run `cargo clippy --lib --features cluster -- -D warnings`

## Verification

### Unit Tests

- `test_pipeline_route_to_nodes` — 3 commands to 2 nodes → 2 batches with correct grouping
- `test_pipeline_response_ordering` — commands sent A→node1, B→node2, C→node1 → results [A, B, C]
- `test_pipeline_execute_single_node` — all commands to one node → behaves like single-node pipeline
- `test_pipeline_execute_multiple_nodes` — commands to 3 nodes → concurrent execution, ordered results

### Integration Tests

- `test_cluster_pipeline_basic` — 9 commands across 3 nodes → 9 results in order
- `test_cluster_pipeline_with_redirect` — MOVED redirect during pipeline → retries on new node
- `test_cluster_pipeline_typed` — typed decode of mixed result types

### Build & Lint

- `cargo build --features cluster` — compiles
- `cargo clippy --lib --features cluster -- -D warnings` — zero warnings
