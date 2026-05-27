# may-redis LLM Wiki Index

## Core

- [Schema](./SCHEMA.md)
- [Log](./log.md)
- [Docs Catalog](./docs-catalog.md)

## Implementation Epics

Epics run in strict order: 0 → 1 → 2 → 3 → 4 → 5 → 6. Each epic's stories must all pass `cargo test` before the next epic begins. See [`docs/Epics/00-epic-overview.md`](./docs/Epics/00-epic-overview.md) for the full plan.

- [Epic 0: Scaffolding](./docs/Epics/epic-0-scaffolding.md) — workspace layout, Cargo.toml, lint config, docs structure
- [Epic 1: Base Crate](./docs/Epics/epic-1-base.md) — RedisValue, RedisError, FromRedisValue, ToRedisArgs
- [Epic 2: Codec Crate](./docs/Epics/epic-2-codec.md) — RESPWriter, RESPReader, encode/decode roundtrip
- [Epic 3: Protocol Crate](./docs/Epics/epic-3-protocol.md) — CommandBuilder, Commands trait, Request/Response tag dispatch
- [Epic 4: Connection Crate](./docs/Epics/epic-4-connection.md) — TcpConnector, epoll loop, request queue, response dispatch
- [Epic 5: Client Crate](./docs/Epics/epic-5-client.md) — RedisClient, Pipeline, InMemoryClient
- [Epic 6: Integration & Migration](./docs/Epics/epic-6-integration.md) — concurrency tests, error handling, redis→may-redis migration

## Reference Topics

- [RESP Protocol Reference](./topics/resp-protocol.md) — RESP wire format, type markers, encoding/decoding algorithms
- [May Coroutine Pattern](./topics/may-coroutine-pattern.md) — may runtime primitives used in may-redis, connection loop architecture
- [Sesame-IDAM Integration](./topics/sesame-idam-integration.md) — Sesame-IDAM Redis usage inventory, command set analysis, migration plan
- [Module Structure](./topics/module-structure.md) — planned modular workspace split, crate dependencies, feature flags

## Reference

- [Codebase Entry Points](./reference/codebase-entry-points.md) — file-level entry points per module
- [Command Mapping](./reference/command-mapping.md) — redis crate → may-redis command translation
