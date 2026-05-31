# Story 15.2 — Redirect Handling (MOVED/ASK)

**Objective:** Parse MOVED and ASK redirect responses from Redis Cluster, update the slot map, and retry commands on the correct node.

**Epic:** 15 — Redis Cluster Support

**Dependencies:** Story 15.1 (ClusterClient connect + basic routing must be complete)

**Source docs:** `docs/PRD-redis-cluster.md` §2.2, §5.2, §5.3, §10.3

## Functional Requirements

- **FR-1: MOVED redirect parsing** — Parse `MOVED slot node` from `RedisValue`
  - Method: `fn parse_moved_redirect(value: &RedisValue) -> Result<(u16, SocketAddr), RedisError>`
  - RESP format: `MOVED 3999 192.168.1.20:6380` returns a bulk string error
  - Extract slot number and target node address
- **FR-2: ASK redirect parsing** — Parse `ASK slot node` from `RedisValue`
  - Method: `fn parse_ask_redirect(value: &RedisValue) -> Result<(u16, SocketAddr), RedisError>`
  - RESP format: `ASK 3999 192.168.1.20:6379` returns a bulk string error
- **FR-3: MOVED retry logic** — On MOVED response, update slot map and retry
  - Update `slot_map[slot] = new_node_id`
  - Get or create connection for new node
  - Retry the command on the new node
  - Permanent redirect: update is permanent, no ASKING needed
- **FR-4: ASK retry logic** — On ASK response, send ASKING then retry
  - Before retry, send `ASKING` command to target node
  - Then send the original command
  - Update slot map once the ASK succeeds
  - Temporary redirect during resharding

## Non-Functional Requirements

- No new dependencies
- Every redirect parse function has unit tests with exact RESP wire formats
- Redirect handling is transparent to the caller — `execute()` just returns the result or error
- Max redirect attempts: 3 (prevent infinite redirect loops)

## Code Anchors

- `src/cluster/redirect.rs` — **NEW** — `parse_moved_redirect()`, `parse_ask_redirect()`, `Redirect` type
- `src/cluster/cluster_client.rs` — existing `execute()` — modify to handle redirects
- `src/cluster/slot_map.rs` — existing `SlotMap::add_node()` — update slot map on redirect
- `src/cluster/mod.rs` — add `pub mod redirect;`, add re-exports
- `src/core/value.rs` — existing `RedisValue` — for parsing redirect errors
- `src/core/error.rs` — existing `RedisError` — new redirect-related error variants

## Structs

```rust
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

/// Error variants for cluster redirect failures.
#[derive(Debug)]
pub enum RedirectError {
    /// Redirect limit exceeded (max 3 attempts).
    MaxRedirectsExceeded,
    /// Could not parse redirect response.
    Parse(String),
    /// Redirect targets an unknown address.
    UnknownNode(SocketAddr),
}
```

## Tasks

- [ ] Add `RedirectKind`, `Redirect` structs to `src/cluster/redirect.rs`
- [ ] Add `RedirectError` enum
- [ ] Implement `parse_moved_redirect()` — parse `MOVED slot addr` from `RedisValue` error
- [ ] Implement `parse_ask_redirect()` — parse `ASK slot addr` from `RedisValue` error
- [ ] Add `RedirectError::MaxRedirectsExceeded` to `RedisError` enum in `src/core/error.rs`
- [ ] Modify `ClusterInner::execute()` in `cluster_client.rs` to detect MOVED/ASK in response
- [ ] Implement MOVED handling: update slot map, get/create connection, retry
- [ ] Implement ASK handling: send ASKING command, retry original command, update slot map
- [ ] Add max redirect depth counter (3 attempts)
- [ ] Wire `redirect` module in `src/cluster/mod.rs`
- [ ] Unit tests: parse MOVED from RESP bulk string error, parse ASK from RESP bulk string error
- [ ] Integration test: redirect command after MOVED response on live cluster
- [ ] Run `cargo build --features cluster`
- [ ] Run `cargo test --lib --features cluster -- --test-threads=1`
- [ ] Run `cargo clippy --lib --features cluster -- -D warnings`

## Verification

### Unit Tests

- `test_parse_moved_valid` — `MOVED 3999 192.168.1.20:6380` → `slot=3999, target=192.168.1.20:6380`
- `test_parse_ask_valid` — `ASK 3999 192.168.1.20:6379` → `slot=3999, target=192.168.1.20:6379`
- `test_parse_non_redirect` — `ERR unknown command` → error (not a redirect)
- `test_moved_update_slot_map` — after MOVED, `slot_map[3999]` points to new node
- `test_ask_retry_with_asking` — ASKING command sent before retry

### Integration Tests

- `test_cluster_redirect_moved` — server returns MOVED, client retries on new node successfully
- `test_cluster_redirect_ask` — server returns ASK, client sends ASKING then retries
- `test_cluster_redirect_max_attempts` — server returns MOVED to non-existent node, max 3 attempts exceeded

### Build & Lint

- `cargo build --features cluster` — compiles
- `cargo clippy --lib --features cluster -- -D warnings` — zero warnings
