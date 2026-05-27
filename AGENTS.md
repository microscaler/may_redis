# may-redis — agent rules

> **Desktop dev environment** — before doing anything in this repo, read the
> Microscaler-wide topology brief. It explains that you are on a Mac but the
> code lives on `ms02` (NFS), where commands execute for this environment, how
> the Kind cluster and vLLM fit in, and the network constraints behind the SSH
> tunneling. Do not duplicate its contents here — link to it. If reality drifts,
> fix the canonical doc, not this copy.
>
> - GitHub: [`cylon-local-infra/docs/desktop-dev-environment.md`](https://github.com/microscaler/cylon-local-infra/blob/main/docs/desktop-dev-environment.md)
> - On ms02 NFS: `~/Workspace/microscaler/cylon-local-infra/docs/desktop-dev-environment.md`

## CRITICAL: Microscaler Dependencies

**We do NOT publish to crates.io. We consume from microscaler forks.**

When analyzing Cargo.toml dependencies, NEVER assume crates.io is the source for any dependency that has a microscaler fork. The crates.io versions are stale or abandoned.

### Microscaler Fork Inventory

| Fork Repo | Upstream | Purpose |
|-----------|----------|---------|
| `microscaler/may` | Xudong-Huang/may | Core stackful coroutine runtime. Foundation of may-redis. |
| `microscaler/may_postgres` | (no direct upstream) | Postgres driver for may coroutines. Reference for may patterns. |
| `microscaler/generator-rs` | Xudong-Huang/generator-rs | Coroutine generator. Patched for Rust 1.90 macOS thread-local bug. |

### Dependency Resolution Rules

- `may` → `version = "0.3"` (uses microscaler fork via workspace resolution)
- `bytes` → `1.7` (standard crate, no microscaler fork)
- `socket2` → `0.5` (standard crate)
- `log` → `0.4` (standard crate)

**Never guess. If you see `may`, `may_postgres`, `generator`, verify the source by checking the Cargo.toml — never assume crates.io.**

---

Strict operational rules for AI assistants working in this repository. **Knowledge about how may-redis works is in [`docs/`](./docs/), not here.** This file only holds rules the agent must obey.

---

## Before you do anything

1. Read [`docs/04-system-design.md`](./docs/04-system-design.md) — the system overview and crate responsibilities.
2. Read [`docs/08-module-structure.md`](./docs/08-module-structure.md) — the planned workspace layout and dependency graph.
3. Read [`docs/01-protocol-analysis.md`](./docs/01-protocol-analysis.md) — RESP wire format reference.
4. Read the specific topic docs relevant to your work (protocol, connection, client, test strategy, migration guide).
5. Drill into source `src/` only when the docs flag drift or a gap.

Sessions that skip this waste work. The docs are the compounding artifact — they cover RESP protocol, may_postgres comparison, sesame-IDAM usage inventory, system design, protocol/connection/client layer design, module structure, migration strategy, test approach, and dependency rationale.

---

## Repository shape

- Primary language: Rust (single crate with modules).
- Flat module layout under `src/`:
  - `base/` — `RedisValue`, `RedisError`, `FromRedisValue`, `ToRedisArgs`
  - `codec/` — RESP encoding/decoding (`RESPWriter`, `RESPReader`)
  - `protocol/` — `CommandBuilder`, `Commands` trait
  - `connection/` — epoll connection loop, TCP, coroutine management
  - `client/` — `RedisClient`, `Pipeline`, public API
- Design docs: 11 numbered markdown files in `docs/` describing the planned modular workspace split (`crates/base/`, `crates/codec/`, etc.). The current code is a single crate; the docs describe the target modular architecture.
- Sibling repo: **`../may_postgres/`** — the may-coroutine Postgres driver. Reference for patterns (connection loop, epoll, request-response). No wiki — documentation lives in `docs/`.

---

## Build, lint, test commands

### Build

- `cargo build` — Build the crate.
- `cargo build --release` — Release build.

### Format / lint

- `cargo fmt` — Format Rust (always run before committing).
- `cargo clippy --workspace --all-targets --all-features` — Lint. Configured deny-lints in `Cargo.toml` (pedantic, nursery, all deny; some allows for cast, module_name_repetitions, struct_excessive_bools, too_many_lines, missing_errors_doc, missing_panics_doc, missing_safety_doc).

### Test

- `cargo test --workspace` — Run all tests.
- `cargo test -p base` — Unit tests only (no runtime, no network).
- `cargo test -p codec` — Unit tests only (no runtime, no network).
- `cargo test -p protocol` — Unit tests with FakeConnection (no network).
- `cargo test -p connection` — Integration tests (requires may runtime).
- `cargo test -p client` — Integration tests (requires may runtime + Redis server).

### Test with may runtime

Since we cannot use `#[tokio::test]`, integration tests use `may::run` to create the coroutine context:

```rust
#[test]
fn test_with_may_runtime() {
    may::run(|| {
        may::go(|| {
            // Test code runs here in a coroutine
        }).join();
    });
}
```

---

## Core rules the agent must obey

### 1. Zero tokio, zero async-await

This is the entire purpose of the project. Every `.await` in new code is a critical bug. All I/O co-operation uses may's `go!`, `co_yield`, `epoll`, `WaitIo`, and `spsc` channels. If you're tempted to add `tokio`, stop and ask why — the answer is always "use may primitives instead."

### 2. Follow Rust conventions

- `snake_case` for fns / modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Group imports: std, external crates, local modules. Prefer explicit imports.
- `Result<T, E>` + `?` over `panic!` in library paths.
- No `unwrap()` / `expect()` in production code (`clippy::unwrap_used` is deny).

### 3. Modular architecture (target state)

The current code is a single crate with flat modules under `src/`. The planned architecture (documented in `docs/08-module-structure.md`) splits into a workspace of 6 crates:

```
base (no may) → codec (no may) → protocol (may) → connection (may+epoll) → client (assembles all) → may-redis (umbrella re-exports)
```

When modifying code, think in terms of module boundaries. Base should have zero may dependency. Codec should have zero may dependency. Only protocol, connection, and client may depend on may.

### 4. RESP2 only (v1 scope)

Only RESP2 type markers are in scope for v1: bulk strings (`$N`), arrays (`*N`), integers (`:N`), errors (`-$N`). RESP3 types (arbitrary binary `~$N`, blob error `=$N`, map `%`, attribute `>`, double `,`, null `_`) are out of scope unless explicitly requested.

### 5. Connection loop pattern

The connection layer follows the may_postgres pattern: a single `go!` coroutine running an epoll loop that:
- Receives commands from application coroutines via an mpsc request queue
- Reads/writes the TCP socket
- Dispatches responses back via spsc channels using a monotonically increasing tag for request-response matching

This is the most may-specific part of the codebase. Changes to the connection loop require understanding epoll event priorities (READABLE vs WRITABLE), non-blocking I/O, and response dispatch ordering.

### 6. API surface must mirror `redis` crate

The goal is mechanical migration from `redis` crate to `may-redis`. The `Commands` trait methods (`get`, `set`, `exists`, `incr`, `del`, `ttl`, `expire`, `publish`, `keys`, `dbsize`, `flushdb`, `ping`, `auth`) should provide a familiar surface. The `cmd()` builder pattern should produce RESP wire format identical to the redis crate.

### 7. Testing discipline

- Unit tests (base, codec, protocol with FakeConnection) run with `#[test]` — no runtime needed.
- Integration tests (connection, client) require may runtime and a real Redis server on localhost:6379.
- Each integration test must call `FLUSHDB` before and after execution for isolation.
- The `InMemoryClient` (feature `test`) provides a clean per-test in-memory backend for unit/integration boundary tests.
- Concurrency tests verify multiple coroutines sharing one client, pipeline ordering, and backpressure behavior.

---

## Commit discipline

- Commits follow Conventional Commits (`feat(scope):`, `fix(scope):`, `docs(scope):`, `chore(scope):`, `refactor(scope):`).
- **Never push** without explicit human authorization.
- **Never use `--no-verify`** or `--no-verify-commit`. Let pre-commit hooks run.
- **Never commit secrets** (`.env`, credentials, tokens).

---

## Useful files

- [`README.md`](./README.md) — project overview.
- [`Cargo.toml`](./Cargo.toml) — workspace config + lint rules.
- [`docs/01-protocol-analysis.md`](./docs/01-protocol-analysis.md) — RESP wire format analysis, comparison with PostgreSQL.
- [`docs/02-may_postgres_comparison.md`](./docs/02-may_postgres_comparison.md) — may_postgres architecture, may primitives used, connection/request-response patterns.
- [`docs/03-sesame-idam-redis-usage.md`](./docs/03-sesame-idam-redis-usage.md) — Sesame-IDAM Redis usage inventory across 5 modules, command frequency analysis.
- [`docs/04-system-design.md`](./docs/04-system-design.md) — System design, crate responsibilities, feature flag matrix, size estimates.
- [`docs/05-protocol-layer-design.md`](./docs/05-protocol-layer-design.md) — RESP encoding/decoding algorithms, type mapping.
- [`docs/06-connection-layer-design.md`](./docs/06-connection-layer-design.md) — Connection loop algorithm, epoll handling, request-response matching, non-blocking I/O.
- [`docs/07-client-api-design.md`](./docs/07-client-api-design.md) — CommandBuilder, Commands trait, Pipeline API, typed results, connection lifecycle.
- [`docs/08-module-structure.md`](./docs/08-module-structure.md) — Planned workspace layout, Cargo.toml per crate, feature flags, build commands, dev workflow.
- [`docs/09-migration-guide.md`](./docs/09-migration-guide.md) — Redis → may-redis migration (6 phases, file-by-file map).
- [`docs/10-test-strategy.md`](./docs/10-test-strategy.md) — Test architecture, test tables per crate, running tests, may runtime setup.
- [`docs/11-dependencies.md`](./docs/11-dependencies.md) — Dependency rationale, comparison with redis/tokio footprint, workspace integration steps.

---

## Explicit instruction: read the docs

**Every session starts with reading the relevant `docs/` pages.** This is not optional. The docs are the comprehensive knowledge base for this project — there is no llmwiki, no wiki index, no log.md. If the docs have a gap or drift, fix them in-place.
