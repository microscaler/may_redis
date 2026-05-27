# May-Redis — Epic Overview

## Project Goal

Build a coroutine-native Redis client for the `may` runtime that replaces the tokio-dependent `redis` crate in sesame-idam. **Zero tokio, zero async-await, only may coroutines.**

## Architecture

```mermaid
graph TB
    subgraph "Application Coroutines"
        App1[App Coroutine 1]
        App2[App Coroutine 2]
    end
    
    subgraph "may-redis Crate"
        subgraph "Phase 0: Scaffolding"
            S[Scaffolding/Workspace]
        end
        
        subgraph "Phase 1: Base"
            B[base — Core Types]
        end
        
        subgraph "Phase 2: Codec"
            C[codec — RESP Encoding/Decoding]
        end
        
        subgraph "Phase 3: Protocol"
            P[protocol — CommandBuilder + Commands]
        end
        
        subgraph "Phase 4: Connection"
            N[connection — epoll loop, TCP]
        end
        
        subgraph "Phase 5: Client"
            L[client — RedisClient + Pipeline]
        end
        
        subgraph "Phase 6: Integration"
            I[integration + migration]
        end
        
        B --> C
        C --> P
        P --> N
        N --> L
        L --> I
        S --> B
        S --> C
        S --> P
    end
    
    App1 -->|cmd().execute()| L
    App2 -->|cmd().execute()| L
```

## Dependency Chain

Each phase builds on the previous one. No phase can complete until its dependencies are implemented.

```mermaid
graph LR
    Epic0[Epic 0<br/>Scaffolding] --> Epic1[Epic 1<br/>base]
    Epic1 --> Epic2[Epic 2<br/>codec]
    Epic2 --> Epic3[Epic 3<br/>protocol]
    Epic3 --> Epic4[Epic 4<br/>connection]
    Epic4 --> Epic5[Epic 5<br/>client]
    Epic5 --> Epic6[Epic 6<br/>integration + migration]
```

## Crate Layout (Target — Epic 0)

```
may_redis/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── base/                     # Epic 1 — Core types (~150 LOC)
│   ├── codec/                    # Epic 2 — RESP encoding/decoding (~300 LOC)
│   ├── protocol/                 # Epic 3 — Command protocol (~400 LOC)
│   ├── connection/               # Epic 4 — Connection loop (~400 LOC)
│   ├── client/                   # Epic 5 — Public client API (~300 LOC)
│   └── may-redis/                # Umbrella re-exports (~50 LOC)
└── docs/
    ├── Epics/                    # Epic + Story definitions (this file)
    ├── 01-protocol-analysis.md   # Reference: RESP wire format
    ├── 02-may_postgres_comparison.md # Reference: may_postgres patterns
    └── 03-sesame-idam-redis-usage.md # Reference: Sesame-IDAM usage inventory
```

## Reference Documentation (Read-Only)

These docs are referenced by multiple epics but are not implementation targets:

- `01-protocol-analysis.md` — RESP wire format, type markers, comparison with PostgreSQL
- `02-may_postgres_comparison.md` — may_postgres architecture as reference implementation
- `03-sesame-idam-redis-usage.md` — Sesame-IDAM Redis usage inventory (command set analysis)

## Execution Rules

1. **Epics run in order** — 0 → 1 → 2 → 3 → 4 → 5 → 6. Each epic's stories must all pass before moving to the next epic.
2. **Each story is independently verifiable** — every story ends with a passing `cargo test`.
3. **Each story documents its code anchors** — explicit file paths and function signatures.
4. **No story is "done" until tested** — unit tests for base/codec/protocol, integration tests for connection/client.
5. **May coroutine only** — never add tokio, async-await, or any other runtime. Reference: `../may_postgres/`.
