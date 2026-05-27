# Module Structure

- Status: unverified
- Source docs: `docs/04-system-design.md`, `docs/08-module-structure.md`, `docs/11-dependencies.md`
- Code anchors: `src/lib.rs`, `src/*/mod.rs`
- Last updated: 2026-05-27

## Current State

Single crate with flat modules under `src/`:

```
may_redis/
├── Cargo.toml
└── src/
    ├── lib.rs        # pub mod base; codec; protocol; connection; client;
    ├── base/mod.rs   # RedisValue, RedisError, FromRedisValue, ToRedisArgs
    ├── codec/mod.rs  # RESPWriter, RESPReader
    ├── protocol/mod.rs # CommandBuilder, Commands trait
    ├── connection/mod.rs # Connection loop, epoll, TCP
    └── client/mod.rs   # RedisClient, Pipeline
```

## Planned Modular Workspace

The docs describe a target architecture splitting into 6 workspace crates:

```
may_redis/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── base/                     # Core types (~150 LOC) — pure data + traits
│   ├── codec/                    # RESP encoding/decoding (~300 LOC)
│   ├── protocol/                 # Command protocol (~400 LOC)
│   ├── connection/               # Connection loop (~400 LOC)
│   ├── client/                   # Public client API (~300 LOC)
│   └── may-redis/                # Umbrella / public API (~50 LOC)
└── docs/                         # Design documents
```

## Crate Dependency Graph

```
base (no may) → codec (no may) → protocol (may) → connection (may+epoll) → client (assembles all) → may-redis (umbrella re-exports)
```

### Crate Responsibilities

| Crate | LOC | Responsibility | External Deps | may Primitives |
|-------|-----|----------------|---------------|----------------|
| `base` | ~150 | RedisValue, RedisError, traits | bytes | none |
| `codec` | ~300 | RESP encoding/decoding | bytes, base | none |
| `protocol` | ~400 | CommandBuilder, Commands trait | bytes, log, may | may::sync::spsc |
| `connection` | ~400 | epoll loop, TCP, coroutine lifecycle | bytes, log, may, socket2 | go!, WaitIo, WaitIoWaker, Queue, spsc |
| `client` | ~300 | RedisClient, Pipeline | base, codec, protocol, connection | — |
| `may-redis` | ~50 | Re-exports, feature flags | all crates | — |

## Feature Flags

| Feature | Default | Purpose |
|---------|---------|---------|
| `base` | always on | Core types — always included |
| `codec` | always on | RESP encoding/decoding — always included |
| `protocol` | always on | Command protocol — always included |
| `connection` | yes | Connection loop + TCP — optional for test-only builds |
| `client` | yes | Public client API — optional for test-only builds |
| `pool` | no | Connection pool support (future) |
| `test` | no | Test helpers (InMemoryClient) |

## External Dependencies

| Crate | Version | Used By | Why |
|-------|---------|---------|-----|
| `bytes` | 1.7 | all crates | BytesMut for buffered I/O, RESP wire format |
| `log` | 0.4 | protocol, connection | Structured logging for connection/protocol errors |
| `may` | 0.3 | protocol, connection, client | The coroutine runtime |
| `socket2` | 0.5 | connection (optional) | Low-level socket configuration |

### NOT Needed

- `tokio` — zero tokio, that's the entire point
- `redis` — we're replacing it
- `async-trait` — no async traits, all methods use may coroutines
- `futures` — may has its own primitives
- `serde` — not needed for RESP, raw bytes and typed extraction
- `rustls`/`native-tls` — no TLS in v1, Redis on localhost or SSH tunnel
