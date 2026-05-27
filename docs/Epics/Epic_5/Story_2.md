# Story 5.2 — Pipeline API

**Objective:** Implement the `Pipeline` struct for batch command execution.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.1 (RedisClient)

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/07-client-api-design.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** `Pipeline::new(client)` creates an empty pipeline backed by a `RedisClient` reference
- [x] **FR-2:** `Pipeline::add(cmd)` encodes a command and appends it to the pipeline
- [x] **FR-3:** `Pipeline::execute<T: FromPipelineResponse>()` sends all commands at once and collects responses
- [x] **FR-4:** Pipeline sends commands in order, responses are collected in order
- [x] **FR-5:** `Pipeline` creates an spsc channel pair per command for response dispatch
- [x] **FR-6:** `FromPipelineResponse` trait for extracting typed results from multiple responses
- [x] **FR-7:** `FromPipelineResponse` implemented for single response `(T1,)`
- [x] **FR-8:** `FromPipelineResponse` implemented for two responses `(T1, T2)`
- [x] **FR-9:** `FromPipelineResponse` implemented for three responses `(T1, T2, T3)`
- [x] **FR-10:** `FromPipelineResponse` implemented for `Vec<T>` — all responses as a typed vec
- [x] **FR-11:** Pipeline commands share a single connection — no separate TCP socket
- [x] **FR-12:** Pipeline handles errors: first error aborts remaining collection

### Non-Functional Requirements

- [x] **NFR-1:** `Pipeline` borrows `RedisClient` — no `Clone`, no `Send` across thread boundaries
- [x] **NFR-2:** No `unwrap()`/`expect()` in production code
- [x] **NFR-3:** Pipeline works within the may coroutine context (no blocking)
- [x] **NFR-4:** Ordering guarantee: commands sent in order, responses received in same order

## Code Anchors

- `src/lib.rs` — `pub use client::pipeline::{FromPipelineResponse, Pipeline};`
- `src/client/pipeline.rs` — `Pipeline` and `FromPipelineResponse` implementation

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

- [x] Define `Pipeline<'a>` struct with client reference, commands vec, and senders vec
- [x] Define `FromPipelineResponse` trait with `from_responses` method
- [x] Implement `Pipeline::new(client: &'a RedisClient) -> Self` — creates empty pipeline
- [x] Implement `Pipeline::add(&mut self, cmd: CommandBuilder)` — encodes and queues command
- [x] Implement `Pipeline::execute<T: FromPipelineResponse>(&mut self) -> Result<T, RedisError>` — sends all, collects responses, decodes
- [x] Implement `FromPipelineResponse` for `(T1,)` — single response
- [x] Implement `FromPipelineResponse` for `(T1, T2)` — two responses
- [x] Implement `FromPipelineResponse` for `(T1, T2, T3)` — three responses
- [x] Implement `FromPipelineResponse` for 4-tuple `(T1, T2, T3, T4)`

## Verification

- `test_integration_pipeline` — pipeline ordering verified end-to-end with real Redis
- Pipeline execute with tuple unpacking: `((), (), got_a): ((), (), Option<String>) = pipe.execute().unwrap()`
- `cargo clippy` — zero warnings
