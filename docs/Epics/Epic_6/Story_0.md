# Epic 6 — Integration & Migration

**Objective:** Full integration testing across all crates, multi-coroutine concurrency testing, and the migration guide from the `redis` crate to `may-redis` for Sesame-IDAM.

**Dependencies:** All previous epics (0-5)

**Source docs:** `docs/09-migration-guide.md`, `docs/10-test-strategy.md`

## Epic Overview

```mermaid
graph TB
    subgraph "Epic 6 — Integration & Migration"
        subgraph "Integration Tests"
            IT1[Multi-corate<br/>concurrency]
            IT2[Pipeline<br/>ordering]
            IT3[Error<br/>propagation]
            IT4[Backpressure<br/>under load]
        end
        
        subgraph "Migration Guide"
            MG1[Phase 1: Drop tokio-comp]
            MG2[Phase 2: Replace imports]
            MG3[Phase 3: Replace connection pattern]
            MG4[Phase 4: Replace query_async]
            MG5[Phase 5: Replace Mutex]
            MG6[Phase 6: Fix test code]
        end
        
        IT1 --> MG1
        IT2 --> MG2
        IT3 --> MG3
        IT4 --> MG4
        MG1 --> MG5
        MG5 --> MG6
    end
```

## Concurrency Model

```mermaid
sequenceDiagram
    participant A1 as App Coroutine 1
    participant A2 as App Coroutine 2
    participant A3 as App Coroutine 3
    participant Q as req_queue<br/>mpsc::Queue
    participant C as Connection Loop<br/>go! + epoll
    participant R as spsc channels
    
    A1->>Q: push Request(tag=1, "GET foo")
    A2->>Q: push Request(tag=2, "SET bar 123")
    A3->>Q: push Request(tag=3, "GET foo")
    
    Note over C: epoll(WRITABLE)
    C->>Q: pop all requests
    C->>R: push Response(tag=1, tx1)
    C->>R: push Response(tag=2, tx2)
    C->>R: push Response(tag=3, tx3)
    C->>C: write all commands to socket
    
    Note over C: epoll(READABLE)
    C->>C: read "*1\r\n$3\r\nbaz\r\n"
    C->>tx1: send BulkString("baz")
    C->>C: read "*1\r\n:1\r\n"
    C->>tx2: send Integer(1)
    C->>C: read "*1\r\n$3\r\nbaz\r\n"
    C->>tx3: send BulkString("baz")
    
    tx1-->>A1: Ok("baz")
    tx2-->>A2: Ok(1)
    tx3-->>A3: Ok("baz")
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 6.1<br/>Full workspace<br/>test pass]
    S1 --> S2[Story 6.2<br/>Concurrency tests]
    S2 --> S3[Story 6.3<br/>Error handling<br/>edge cases]
    S3 --> S4[Story 6.4<br/>Migration guide<br/>documentation]
    S4 --> PASS[Epic complete<br/>all stories verified]
```
