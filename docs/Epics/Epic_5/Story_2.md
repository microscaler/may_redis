# Story 5.2 — Pipeline API

**Objective:** Implement the `Pipeline` struct for batch command execution.

**Epic:** 5 — Client Crate

**Dependencies:** Story 5.1

**Source docs:** `docs/07-client-api-design.md`

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

## Tasks

1. Define `Pipeline<'a>` with commands vec and senders vec
2. Define `FromPipelineResponse` trait for extracting multiple responses
3. Implement `Pipeline::new(client)` — creates empty pipeline
4. Implement `add(cmd)` — encodes command, pushes to commands vec, creates spsc pair for response
5. Implement `execute<T: FromPipelineResponse>()` — the full pipeline flow:
   - Send all commands to connection queue (no waiting)
   - Read responses in order from spsc channels
   - Decode each response using FromPipelineResponse
   - Return typed result tuple
6. Implement `FromPipelineResponse` for `(T1,)` — single response
7. Implement `FromPipelineResponse` for `(T1, T2)` — two responses
8. Implement `FromPipelineResponse` for `(T1, T2, T3)` — three responses
9. Implement `FromPipelineResponse` for `Vec<T>` — all responses extracted as Vec

## Verification

- `cargo test -p client` — at least 5 unit tests:
  - `test_pipeline_creation` — Pipeline::new() creates empty pipeline
  - `test_pipeline_add_command` — add command, verify commands vec has 1 element
  - `test_pipeline_add_multiple` — add 3 commands, verify ordering preserved
  - `test_pipeline_execute_single` — execute 1 command, verify response
  - `test_pipeline_execute_multiple` — execute 3 commands, verify 3 responses in order
- `cargo clippy -p client` — zero warnings
