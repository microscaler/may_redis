# Story 5.2 — Pipeline API

**Objective:** Implement the `Pipeline` struct for batch command execution.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.1 (RedisClient)

**Source docs:** `docs/07-client-api-design.md`

## Requirements

### Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| FR-1 | `Pipeline::new(client)` creates an empty pipeline backed by a `RedisClient` reference | P0 |
| FR-2 | `Pipeline::add(cmd)` encodes a command and appends it to the pipeline | P0 |
| FR-3 | `Pipeline::execute<T: FromPipelineResponse>()` sends all commands at once and collects responses | P0 |
| FR-4 | Pipeline sends commands in order, responses are collected in order | P0 |
| FR-5 | `Pipeline` creates an spsc channel pair per command for response dispatch | P0 |
| FR-6 | `FromPipelineResponse` trait for extracting typed results from multiple responses | P0 |
| FR-7 | `FromPipelineResponse` implemented for single response `(T1,)` | P1 |
| FR-8 | `FromPipelineResponse` implemented for two responses `(T1, T2)` | P1 |
| FR-9 | `FromPipelineResponse` implemented for three responses `(T1, T2, T3)` | P1 |
| FR-10 | `FromPipelineResponse` implemented for `Vec<T>` — all responses as a typed vec | P2 |
| FR-11 | Pipeline commands share a single connection — no separate TCP socket | P1 |
| FR-12 | Pipeline handles errors: first error aborts remaining collection | P1 |

### Non-Functional Requirements

| # | Requirement | Priority |
|---|---|---|
| NFR-1 | `Pipeline` borrows `RedisClient` — no `Clone`, no `Send` across thread boundaries | P0 |
| NFR-2 | No `unwrap()`/`expect()` in production code | P1 |
| NFR-3 | Pipeline must work within the may coroutine context (no blocking) | P0 |
| NFR-4 | Ordering guarantee: commands sent in order, responses received in same order | P0 |

## Code Anchors

- `crates/client/src/lib.rs` — `pub struct Pipeline`
- `crates/client/src/pipeline.rs` — implementation

## Structs

```rust
pub struct Pipeline<'a> {
    client: &'a RedisClient,
    commands: Vec<BytesMut>,
    senders: Vec<spsc::Receiver<RedisValue>>,
}

pub trait FromPipelineResponse {
    fn from_responses(responses: Vec<RedisValue>) -> Result<Self, RedisError>
    where Self: Sized;
}
```

## Implementation Tasks

- [ ] Define `Pipeline<'a>` struct with client reference, commands vec, and senders vec
- [ ] Define `FromPipelineResponse` trait with `from_responses` method
- [ ] Implement `Pipeline::new(client: &'a RedisClient) -> Self` — creates empty pipeline
- [ ] Implement `Pipeline::add(&mut self, cmd: CommandBuilder)`:
  - [ ] Encode command using `RESPWriter` into `BytesMut`
  - [ ] Push encoded bytes to `commands` vec
  - [ ] Create spsc channel pair, store sender in pending
  - [ ] Create `Request` and push to connection's queue
  - [ ] Store receiver in `senders` vec
- [ ] Implement `Pipeline::execute<T: FromPipelineResponse>(&mut self) -> Result<T, RedisError>`:
  - [ ] All commands already sent (via `add`)
  - [ ] Read responses in order from `senders` vec
  - [ ] Collect `RedisValue` responses into a `Vec`
  - [ ] Decode using `FromPipelineResponse::from_responses`
  - [ ] Return `Result<T, RedisError>`
- [ ] Implement `FromPipelineResponse` for `(T1,)` — single response
- [ ] Implement `FromPipelineResponse` for `(T1, T2)` — two responses
- [ ] Implement `FromPipelineResponse` for `(T1, T2, T3)` — three responses
- [ ] Implement `FromPipelineResponse` for `Vec<T>` — all responses as typed vec

## Verification

### Unit Tests (minimum 5)

- [ ] `test_pipeline_creation` — `Pipeline::new()` creates empty pipeline with zero commands
- [ ] `test_pipeline_add_command` — add one command, verify `commands.len() == 1`
- [ ] `test_pipeline_add_multiple` — add 3 commands, verify ordering preserved
- [ ] `test_pipeline_execute_single` — execute 1 command, verify response
- [ ] `test_pipeline_execute_multiple` — execute 3 commands, verify 3 responses in order
- [ ] `test_pipeline_error_handling` — error response is propagated

### Lint & Build

- [ ] `cargo test -p client` — all tests pass
- [ ] `cargo clippy -p client` — zero warnings
- [ ] `cargo fmt -p client` — formatted
