# Epic 2 — Codec Crate

**Objective:** Implement the RESP encoding/decoding codec. This crate depends on `base` + `bytes` but **still has no may dependency**. Pure data transformation — testable with plain `#[test]`.

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base)

**Source docs:** `docs/01-protocol-analysis.md`, `docs/05-protocol-layer-design.md`

## Crate Overview

```mermaid
graph TB
    subgraph "codec crate — no may, pure encoding/decoding"
        RW[RESPWriter<br/>write_simple<br/>write_bulk<br/>write_int<br/>write_array<br/>take]
        RR[RESPReader<br/>read_value<br/>read_marker<br/>read_length]
        
        Args[Rust Args<br/>RedisValue] --> RW
        RW --> Wire[BytesMut<br/>RESP wire format]
        Wire --> RR
        RR --> Native[Rust Types<br/>RedisValue]
        
        RW -. uses.-> Base[base crate]
        RR -. uses.-> Base
    end
    
    subgraph "External deps"
        Bytes[bytes — BytesMut]
    end
    
    Bytes -. used by.-> RW
    Bytes -. used by.-> RR
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 2.1<br/>RESPWriter<br/>encode direction]
    S1 --> S2[Story 2.2<br/>RESPReader<br/>decode direction]
    S2 --> S3[Story 2.3<br/>Full RESP2<br/>+ roundtrip tests]
    S3 --> PASS[All tests pass<br/>cargo test -p codec]
```
