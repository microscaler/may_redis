# Docs Catalog (Initial Ingest)

- Status: partially-verified
- Scope: full `docs/**/*.md` inventory (11 files)

## Inventory

| File | Purpose |
|------|---------|
| [`docs/01-protocol-analysis.md`](./docs/01-protocol-analysis.md) | RESP wire format analysis, comparison with PostgreSQL wire protocol |
| [`docs/02-may_postgres_comparison.md`](./docs/02-may_postgres_comparison.md) | may_postgres architecture, may primitives used, connection/request-response patterns |
| [`docs/03-sesame-idam-redis-usage.md`](./docs/03-sesame-idam-redis-usage.md) | Sesame-IDAM Redis usage across 5 modules, command frequency analysis |
| [`docs/04-system-design.md`](./docs/04-system-design.md) | System design, crate responsibilities, feature flag matrix, size estimates |
| [`docs/05-protocol-layer-design.md`](./docs/05-protocol-layer-design.md) | RESP encoding/decoding algorithms, type mapping, encoder/decoder flow |
| [`docs/06-connection-layer-design.md`](./docs/06-connection-layer-design.md) | Connection loop algorithm, epoll handling, request-response matching, non-blocking I/O |
| [`docs/07-client-api-design.md`](./docs/07-client-api-design.md) | CommandBuilder, Commands trait, Pipeline API, typed results, connection lifecycle |
| [`docs/08-module-structure.md`](./docs/08-module-structure.md) | Planned workspace layout, Cargo.toml per crate, feature flags, build commands, dev workflow |
| [`docs/09-migration-guide.md`](./docs/09-migration-guide.md) | Redis → may-redis migration (6 phases, file-by-file map) |
| [`docs/10-test-strategy.md`](./docs/10-test-strategy.md) | Test architecture, test tables per crate, running tests, may runtime setup |
| [`docs/11-dependencies.md`](./docs/11-dependencies.md) | Dependency rationale, comparison with redis/tokio footprint, workspace integration |

## LLM wiki synthesis index (initial)

| Wiki page | Maps to |
|-----------|---------|
| [`topics/resp-protocol.md`](./topics/resp-protocol.md) | `docs/01-protocol-analysis.md`, `docs/05-protocol-layer-design.md` |
| [`topics/may-coroutine-pattern.md`](./topics/may-coroutine-pattern.md) | `docs/02-may_postgres_comparison.md`, `docs/06-connection-layer-design.md` |
| [`topics/sesame-idam-integration.md`](./topics/sesame-idam-integration.md) | `docs/03-sesame-idam-redis-usage.md`, `docs/09-migration-guide.md` |
| [`topics/module-structure.md`](./topics/module-structure.md) | `docs/04-system-design.md`, `docs/08-module-structure.md`, `docs/11-dependencies.md` |
| [`reference/codebase-entry-points.md`](./reference/codebase-entry-points.md) | `src/lib.rs`, `src/*/mod.rs` |
| [`reference/command-mapping.md`](./reference/command-mapping.md) | `docs/07-client-api-design.md`, `docs/03-sesame-idam-redis-usage.md` |
