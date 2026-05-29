# may-redis Documentation Index

> A unified catalog of all documentation. Start here to find what you need.

## Getting Started

- [architecture.md](./architecture.md) — Full crate overview, runtime diagram, module layout, error types, public API tour, project roadmap. The single best place to start.
- [migration-guide.md](./migration-guide.md) — How to migrate from the `redis` crate to `may-redis`. API differences, trait method mappings, pipeline patterns.

## Architecture & Design

- [adr-001-single-crate-structure.md](./adr-001-single-crate-structure.md) — Why a single crate, not a workspace. Trade-offs reviewed.
- [test-strategy.md](./test-strategy.md) — Two-tier testing: unit tests (no runtime) vs integration tests (may runtime + live Redis). Test tables per crate.
- [09-migration-guide.md](./09-migration-guide.md) — Legacy path for migration-guide.md (redirect to migration-guide.md).

## Protocol & Wire Format

- [01-protocol-analysis.md](./01-protocol-analysis.md) — RESP2 wire format reference: type markers, encoding rules, error handling. Comparison with PostgreSQL.
- [02-may_postgres_comparison.md](./02-may_postgres_comparison.md) — How may-redis mirrors may-postgres patterns: connection loop, epoll, request-response pipeline, may primitives.
- [03-sesame-idam-redis-usage.md](./03-sesame-idam-redis-usage.md) — Sesame-IDAM Redis command inventory: 5 modules, 11 canonical commands, frequency analysis.

## Implementation Stories

- [Epics/](./Epics/) — Implementation roadmap organized as epics and stories.
  - **Epic 0** — Project foundation: `Cargo.toml`, module structure, RESP codec, `RedisValue`, `RedisClient` skeleton
  - **Epic 1** — Basic commands: `GET`, `SET`, `DEL`, `PING`, `EXISTS`, `TTL`, `EXPIRE`
  - **Epic 2** — Pipeline support
  - **Epic 3** — Connection loop hardening, in-memory test backend, SSRF protection, command policy
  - **Epic 4** — Pub/Sub support
  - **Epic 5** — Advanced commands (HASH, SET, LIST, SORTED SET)
  - **Epic 6** — Connection pooling
  - **Epic 7** — Redis command expansion (String Extension, HASH, SET, LIST, SORTED SET, etc.)
  - **Epic 8+** — Additional features (cluster, sentinel, TLS — out of scope for v1)

Each epic has `Story_0.md` (overview with architecture diagrams) and `Story_1..N.md` (granular implementation stories with code anchors, tasks, and verification).

## Security

- [SSRF Protection](../llmwiki/concepts/ssrf-protection.md) — SSRF guard architecture: blocks connections to private, link-local, loopback, reserved IPs after DNS resolution. Requirements FR-025/FR-027/FR-028.
- [Command Policy](../llmwiki/concepts/command-policy.md) — Enum-based command-level access control: AllowAll, DenyCommands, AllowCommands. Pre-build validation. Requirements FR-029/FR-030/FR-031/FR-032.

## Reference & Analysis

- [redis-implementation-audit.md](./redis-implementation-audit.md) — Comprehensive audit of Redis commands: which are implemented, which are missing, coverage analysis.
- [perf-test-plan.md](./perf-test-plan.md) — Performance testing plan for may-redis benchmarks.
- [JSF_COMPLIANCE.md](./JSF_COMPLIANCE.md) — JSF-AV rule compliance audit: AV1, AV3, AV206, AV208, AV119, AV148/209.
- [JSF_AUDIT_2026_05_28.md](./JSF_AUDIT_2026_05_28.md) — Detailed JSF-AV audit report.
- [code-review-2026-05-28.md](./code-review-2026_05_28.md) — Full codebase expert review with findings.

## Contributing

- [../CONTRIBUTING.md](../CONTRIBUTING.md) — Project conventions, coding standards, commit rules, and how to contribute to may-redis.

## Knowledge Base

- [../llmwiki/index.md](../llmwiki/index.md) — The full llmwiki index with all entity, concept, comparison, and topic pages. Updated with SSRF protection and command policy documentation.
