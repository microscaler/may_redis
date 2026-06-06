# may-redis Documentation Index

> A unified catalog of all documentation. Start here to find what you need.

## Getting Started

- [architecture.md](./architecture.md) — Full crate overview, runtime diagram, module layout, error types, public API tour, epics status, project roadmap. The single best place to start.
- [migration-guide.md](./migration-guide.md) — How to migrate from the `redis` crate to `may-redis`. API differences, trait method mappings, pipeline patterns.

## Architecture & Design

- [adr-001-single-crate-structure.md](./adr-001-single-crate-structure.md) — Why a single crate, not a workspace. Trade-offs reviewed.
- [test-strategy.md](./test-strategy.md) — Two-tier testing: unit tests (no runtime) vs integration tests (may runtime + live Redis). Docker fixture architecture.
- [09-migration-guide.md](./09-migration-guide.md) — Legacy path for migration-guide.md (redirect to migration-guide.md).

## Protocol & Wire Format

- [01-protocol-analysis.md](./01-protocol-analysis.md) — RESP2 wire format reference: type markers, encoding rules, error handling. Comparison with PostgreSQL.
- [02-may_postgres_comparison.md](./02-may_postgres_comparison.md) — How may-redis mirrors may-postgres patterns: connection loop, epoll, request-response pipeline, may primitives.
- [03-sesame-idam-redis-usage.md](./03-sesame-idam-redis-usage.md) — Sesame-IDAM Redis command inventory: 5 modules, 11 canonical commands, frequency analysis.

## Implementation Stories

- [Epics/](./Epics/) — Implementation roadmap organized as epics and stories.
  - **Epic 0** — Project foundation: `Cargo.toml`, module structure, RESP codec, `RedisValue`, `RedisClient` skeleton — **COMPLETE**
  - **Epic 1** — Core types: `RedisValue`, `RedisError`, `FromRedisValue`, `ToRedisArgs`, `f64/u64/i32/u8` impls — **COMPLETE**
  - **Epic 2** — RESP codec: `RESPReader`, `RESPWriter`, roundtrip tests, CRLF handling, depth/length caps — **COMPLETE**
  - **Epic 3** — Protocol layer: `CommandBuilder`, `Commands` trait, `FakeConnection`, command policy — **COMPLETE**
  - **Epic 4** — Connection loop: epoll loop, request-response pipeline, TCP connector, `may` primitives — **COMPLETE**
  - **Epic 5** — Client API: `RedisClient`, `Pipeline`, `InMemoryClient` — **COMPLETE**
  - **Epic 6** — Integration tests: multi-coroutine concurrency, error handling, migration guide — **COMPLETE**
  - **Epic 7** — Command expansion: ~96 commands across 9 categories (String, Hash, Set, List, Sorted Set, Server, Transaction, Pub/Sub, General) — **COMPLETE**
  - **Epic 8** — Implementation gaps: `FromRedisValue` for additional types, dead code removal, connection robustness — **COMPLETE**
  - **Epic 9** — JSF-AV compliance: lint profile, no-panic dispatch, bounded complexity, explicit types — **COMPLETE**
  - **Epic 10** — Docs & lints: rustdocs on all public interfaces, deny(lints) — **COMPLETE**
  - **Epic 11** — Code review remediation: URL parsing, timeouts, command policy, SSRF — **IN PROGRESS**
  - **Epic 12** — Regression tests: edge-case tests for Epic 11 findings — **IN PROGRESS**
  - **Epic 13** — Security audit: SSRF, command injection, resource limits, memory safety — **IN PROGRESS**
  - **Epic 14** — TLS/mTLS: rustls 0.23, `rediss://` URL parsing, client/server certs, handshake, 60+ tests — **IN PROGRESS**
  - **Epic 15** — Redis Cluster: CRC16, hash-tag extraction, slot map, topology, fan-out, redirect handling, 27+ tests — **IN PROGRESS**
  - **Epic 16** — Docker fixtures: bollard containers, `shared_fixture()`, plain + TLS, skip-docker — **IN PROGRESS**

Each epic has `Story_0.md` (overview with architecture diagrams) and `Story_1..N.md` (granular implementation stories with code anchors, tasks, and verification).

## Security

- [SSRF Protection](../llmwiki/concepts/ssrf-protection.md) — SSRF guard architecture: blocks connections to private, link-local, loopback, reserved IPs after DNS resolution.
- [Command Policy](../llmwiki/concepts/command-policy.md) — Enum-based command-level access control: `AllowAll`, `DenyCommands`, `AllowCommands`. Pre-build validation.
- [TLS/mTLS Architecture](./PRD_TLS_mTLS.md) — TLS support implementation plan: `rediss://` URL parsing, server cert verification, client certificates, connection methods.

## Reference & Analysis

- [redis-implementation-audit.md](./redis-implementation-audit.md) — Comprehensive audit of Redis commands: which are implemented, which are missing, coverage analysis.
- [command-coverage-audit.md](./command-coverage-audit.md) — Command-by-command coverage audit with trait method mappings.
- [perf-test-plan.md](./perf-test-plan.md) — Performance testing plan for may-redis benchmarks.
- [PRD-file-breakdown-audit.md](./PRD-file-breakdown-audit.md) — Oversized file audit and modularization plan.
- [PRD-redis-cluster.md](./PRD-redis-cluster.md) — Redis Cluster support PRD: hash-slot routing, MOVED/ASK redirects, topology discovery.
- [PRD-connection-concurrency.md](./PRD-connection-concurrency.md) — Connection concurrency patterns and benchmarks.
- [PRD-e2e-test-coverage.md](./PRD-e2e-test-coverage.md) — E2E test coverage audit and implementation plan.
- [PRD-bollard-test-containers.md](./PRD-bollard-test-containers.md) — Bollard Docker fixture implementation plan.
- [JSF_COMPLIANCE.md](./JSF_COMPLIANCE.md) — JSF-AV rule compliance audit: AV1, AV3, AV206, AV208, AV119, AV148/209.
- [JSF_AUDIT_2026_05_28.md](./JSF_AUDIT_2026_05_28.md) — Detailed JSF-AV audit report.
- [code-review-2026-05-28.md](./code-review-2026_05_28.md) — Full codebase expert review with findings.
- [module-audit.md](./module-audit.md) — Module boundary analysis and file-size audit.

## Contributing

- [../CONTRIBUTING.md](../CONTRIBUTING.md) — Project conventions, coding standards, commit rules, and how to contribute to may-redis.

## Knowledge Base

- [../llmwiki/index.md](../llmwiki/index.md) — The full llmwiki index with all entity, concept, comparison, and topic pages.
- [../llmwiki/log.md](../llmwiki/log.md) — Development log tracking epics, fixes, and architectural decisions.
- [../llmwiki/docs-catalog.md](../llmwiki/docs-catalog.md) — Structured document catalog synced with INDEX.md.
