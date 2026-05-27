# Epic 3 — Protocol Crate

**Objective:** Implement the command protocol layer — CommandBuilder fluent API, Commands trait, and Request/Response management. This is the first crate that depends on `may` (for spsc channels).

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec)

**Source docs:** `docs/04-system-design.md`, `docs/05-protocol-layer-design.md`, `docs/07-client-api-design.md`

## Crate Overview

```mermaid
graph TB
    subgraph "protocol crate — depends on may"
        CB[CommandBuilder<br/>new cmd<br/>arg val<br/>build]
        CT[Commands trait<br/>get set exists<br/>incr del ttl<br/>expire publish<br/>keys dbsize<br/>flushdb ping auth]
        REQ[Request<br/>tag + BytesMut<br/>+ spsc sender]
        RESP[Response<br/>tag + spsc receiver]
        Tags[AtomicTag counter<br/>monotonic tag gen]
        
        CT --> CB
        CB --> REQ
        REQ --> Tags
        RESP --> Tags
    end
    
    subgraph "External deps"
        Bytes[bytes]
        Log[log]
        May[may — spsync channels]
    end
    
    Bytes -. used by.-> CB
    Log -. used by.-> CB
    May -. used by.-> REQ
    May -. used by.-> RESP
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 3.1<br/>CommandBuilder]
    S1 --> S2[Story 3.2<br/>Commands trait]
    S2 --> S3[Story 3.3<br/>Request + tag dispatch]
    S3 --> S4[Story 3.4<br/>Integration<br/>encode + spsc send]
    S4 --> PASS[All tests pass<br/>cargo test -p protocol]
```
