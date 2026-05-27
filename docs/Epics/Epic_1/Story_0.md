# Epic 1 — Base Crate

**Objective:** Implement the core Redis data types and conversion traits. This crate has **no may dependency**, **no network dependency**, and can be tested with plain `#[test]`. It is the foundation of everything else.

**Dependencies:** Epic 0 (scaffolding)

**Source docs:** `docs/08-module-structure.md`, `docs/11-dependencies.md`

## Crate Overview

```mermaid
graph TB
    subgraph "base crate — no may, no network, no IO"
        RV[RedisValue enum<br/>BulkString / Array / Integer<br/>SimpleString / Error / Null]
        RE[RedisError enum<br/>Connection / Protocol / Parse]
        FRV[FromRedisValue trait<br/>Extract Rust types from RedisValue]
        TRA[ToRedisArgs trait<br/>Convert Rust types to RedisValue]
        
        RV --> FRV
        RE --> FRV
        RV --> TRA
    end
    
    subgraph "External deps"
        Bytes[bytes — BytesMut]
    end
    
    Bytes -. used by.-> RV
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 1.1<br/>RedisValue enum]
    S1 --> S2[Story 1.2<br/>RedisError + FromRedisValue]
    S2 --> S3[Story 1.3<br/>ToRedisArgs trait]
    S3 --> S4[Story 1.4<br/>Full FromRedisValue impls]
    S4 --> PASS[All tests pass<br/>cargo test -p base]
```
