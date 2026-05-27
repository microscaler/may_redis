# Epic 5 — Client Crate

**Objective:** Implement the user-facing API — `RedisClient` for single commands, `Pipeline` for batch commands, and `InMemoryClient` for test isolation.

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec) + Epic 3 (protocol) + Epic 4 (connection)

**Source docs:** `docs/07-client-api-design.md`

## Crate Overview

```mermaid
graph TB
    subgraph "client crate — assembles all layers"
        RC[RedisClient<br/>connect<br/>execute<br/>pipeline]
        PL[Pipeline<br/>add command<br/>execute batch]
        IC[InMemoryClient<br/>feature=test<br/>in-memory backend]
        IC2[InMemoryStore<br/>key-value store<br/>TTL support]
        
        RC --> Conn[connection crate]
        RC --> Proto[protocol crate]
        PL --> Proto
        RC --> PL
        IC --> IC2
    end
    
    subgraph "External deps"
        All[base + codec + protocol + connection]
    end
    
    All -. all.-> RC
    All -. all.-> PL
    All -. all.-> IC
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 5.1<br/>RedisClient<br/>connect + execute]
    S1 --> S2[Story 5.2<br/>Pipeline API]
    S2 --> S3[Story 5.3<br/>InMemoryClient<br/>test backend]
    S3 --> PASS[All tests pass<br/>cargo test -p client]
```
