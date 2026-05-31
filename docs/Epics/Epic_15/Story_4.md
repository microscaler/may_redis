# Story 15.4 — Multi-Key Fan-Out

**Objective:** Handle commands with multiple keys that span different hash slots — fan out to multiple nodes, aggregate responses, and return combined results.

**Epic:** 15 — Redis Cluster Support

**Dependencies:** Story 15.1 (ClusterClient), Story 15.2 (Redirect handling), Story 15.3 (Topology discovery)

**Source docs:** `docs/PRD-redis-cluster.md` §2.1, §5.4, §10.5

## Functional Requirements

- **FR-1: Multi-key extraction** — Extract all keys from commands that support multiple keys
  - Commands: `DEL key [key ...]`, `MSET key value [key value ...]`, `SMOVE src dst member`
  - Method: `fn extract_keys(&self, cmd: &CommandBuilder) -> Result<Vec<&[u8]>, RedisError>`
  - DEL: all args are keys
  - MSET: odd-indexed args are keys (arg 0, 2, 4, ...)
  - SMOVE: first arg is source key, second is destination key
- **FR-2: Same-slot verification** — Check if all keys map to the same slot
  - Method: `fn keys_same_slot(&self, keys: &[&[u8]]) -> Result<u16, RedisError>`
  - Compute slot for each key, return common slot if all same
  - Return error if keys span multiple slots
- **FR-3: Cross-slot fan-out** — Split multi-key commands by target node
  - Method: `fn fan_out(&self, cmd: &CommandBuilder) -> Result<Vec<FanOutCommand>, RedisError>`
  - Group keys by target slot → create separate sub-commands per node
  - Each sub-command contains the RESP bytes for that node's portion
- **FR-4: Response aggregation** — Collect and aggregate responses from multiple nodes
  - Method: `async fn aggregate_responses(&self, results: Vec<Result<RedisValue, RedisError>>) -> Result<RedisValue, RedisError>`
  - DEL: sum of individual results
  - MSET: return single OK (all succeeded)
  - SMOVE: return integer (1 = success, 0 = failure)
- **FR-5: Command validation** — Reject commands that cannot be fanned out
  - Commands like `MGET` with cross-slot keys: fan out and return combined result
  - Commands that must be atomic: reject with `RedisError::Cluster("command not supported across slots")`

## Non-Functional Requirements

- Fan-out uses may coroutines: each sub-command runs in its own coroutine
- Responses are collected via `may::sync::spsc::Receiver`
- Timeout applies to the entire fan-out operation, not individual commands
- If ANY sub-command fails, return the first error (fail-fast)

## Code Anchors

- `src/cluster/fanout.rs` — **NEW** — `extract_keys()`, `keys_same_slot()`, `fan_out()`, `aggregate_responses()`
- `src/cluster/cluster_client.rs` — existing `execute()` — route to fan-out for multi-key commands
- `src/cluster/mod.rs` — add `pub mod fanout;`, add re-exports
- `src/protocol/builder.rs` — existing `CommandBuilder` — for extracting args and building sub-commands
- `src/cluster/crc16.rs` — existing `compute_slot()` — for computing per-key slots
- `src/cluster/slot_map.rs` — existing `SlotMap::node_for_slot()` — for looking up target nodes

## Structs

```rust
/// A single sub-command for fan-out execution.
#[derive(Debug)]
pub struct FanOutCommand {
    /// The RESP-encoded command bytes.
    pub data: Vec<u8>,
    /// The target node's Connection.
    pub connection: Connection,
    /// The slot this command targets (for tracking).
    pub slot: u16,
}

/// Error for fan-out failures.
#[derive(Debug)]
pub enum FanOutError {
    /// All keys must map to the same slot for this command.
    CrossSlotUnsupported(String),
    /// One or more sub-commands failed.
    PartialFailure {
        successful: usize,
        failed: usize,
        first_error: RedisError,
    },
}
```

## Tasks

- [ ] Add `FanOutCommand`, `FanOutError` structs
- [ ] Implement `extract_keys()` for DEL, MSET, SMOVE, MGET
- [ ] Implement `keys_same_slot()` — verify all keys map to one slot
- [ ] Implement `fan_out()` — split multi-key command into per-node sub-commands
- [ ] Implement `aggregate_responses()` — combine results from multiple nodes
- [ ] Modify `ClusterClient::execute()` to detect multi-key commands and route to fan-out
- [ ] Add command validation: reject unsupported cross-slot commands
- [ ] Wire `fanout` module in `src/cluster/mod.rs`
- [ ] Unit tests: extract keys from DEL, MSET, SMOVE
- [ ] Unit tests: same-slot verification (all same → slot number, different → error)
- [ ] Unit tests: fan-out for DEL across 2 slots → 2 sub-commands
- [ ] Unit tests: response aggregation for DEL (sum of ints), MSET (single OK)
- [ ] Integration test: DEL keys across 3 nodes → aggregated result
- [ ] Integration test: MGET keys across 3 nodes → aggregated bulk string array
- [ ] Run `cargo build --features cluster`
- [ ] Run `cargo test --lib --features cluster -- --test-threads=1`
- [ ] Run `cargo clippy --lib --features cluster -- -D warnings`

## Verification

### Unit Tests

- `test_extract_keys_del` — `DEL key1 key2 key3` → `[[key1], [key2], [key3]]`
- `test_extract_keys_mset` — `MSET k1 v1 k2 v2` → `[[k1], [k2]]` (keys only)
- `test_extract_keys_smvove` — `SMOVE src dst member` → `[[src], [dst]]`
- `test_keys_same_slot_all_same` — 3 keys all map to slot 100 → `Ok(100)`
- `test_keys_same_slot_different` — keys map to slots 100 and 200 → `Err(CrossSlotUnsupported)`
- `test_fan_out_del_3_slots` — DEL 3 keys on 3 slots → 3 FanOutCommands
- `test_aggregate_del_results` — 3 DEL responses (1, 0, 1) → RedisValue::Integer(2)
- `test_aggregate_mset_results` — 2 MSET responses (OK, OK) → RedisValue::BulkString("OK")

### Integration Tests

- `test_del_cross_slot` — DEL keys spanning 2 nodes → both succeed, sum = 2
- `test_mget_cross_slot` — MGET keys spanning 3 nodes → 3 bulk strings returned
- `test_mset_cross_slot` — MSET keys spanning 2 nodes → single OK
- `test_fan_out_partial_failure` — one node fails → return first error with context

### Build & Lint

- `cargo build --features cluster` — compiles
- `cargo clippy --lib --features cluster -- -D warnings` — zero warnings
