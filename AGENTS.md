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

1. Read [`docs/Epics/00-epic-overview.md`](./docs/Epics/00-epic-overview.md) — project goal, architecture, and epic plan.
2. Read the specific epic for your task:
   - **Epic 0** (scaffolding): [`docs/Epics/epic-0-scaffolding.md`](./docs/Epics/epic-0-scaffolding.md)
   - **Epic 1** (base): [`docs/Epics/epic-1-base.md`](./docs/Epics/epic-1-base.md)
   - **Epic 2** (codec): [`docs/Epics/epic-2-codec.md`](./docs/Epics/epic-2-codec.md)
   - **Epic 3** (protocol): [`docs/Epics/epic-3-protocol.md`](./docs/Epics/epic-3-protocol.md)
   - **Epic 4** (connection): [`docs/Epics/epic-4-connection.md`](./docs/Epics/epic-4-connection.md)
   - **Epic 5** (client): [`docs/Epics/epic-5-client.md`](./docs/Epics/epic-5-client.md)
   - **Epic 6** (integration + migration): [`docs/Epics/epic-6-integration.md`](./docs/Epics/epic-6-integration.md)
3. Read `docs/01-protocol-analysis.md` — RESP wire format reference.
4. Read `docs/02-may_postgres_comparison.md` — may_postgres reference patterns.
5. Read `docs/03-sesame-idam-redis-usage.md` — Sesame-IDAM command set inventory.
6. Drill into source `src/` only when the epics flag drift or a gap.

Epics run in strict order: 0 → 1 → 2 → 3 → 4 → 5 → 6. Each epic's stories must all pass `cargo test` before moving to the next epic. The epics are the compounding artifact — they decompose the design docs into verifiable, ordered, independently testable implementation stories.

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

### 1. This is a may coroutine project — NO tokio, NO async-await, NO other runtimes

may-redis is a coroutine-native Redis client built exclusively on the `may` runtime. This is not optional, not a preference, and not subject to debate — it is the entire raison d'être of the project.

**Absolute constraints:**
- **Never add `tokio`** to Cargo.toml. Never import `tokio::sync`, `tokio::net`, `tokio::time`, `tokio::macros`, or any tokio crate.
- **Never use `.await`** anywhere in the codebase. Every `.await` is a critical bug.
- **Never add alternative runtimes** (async-std, smol, quanta, etc.). The only allowed runtime is `may`.
- **Never suggest tokio-compatible patterns** (async traits, futures combinators, Pin, BoxFuture, Stream, Sink).
- **Never reach for `#[tokio::test]`**, `#[tokio::main]`, `async fn`, or `async {}` blocks.

**All I/O co-operation uses may primitives:**
- `may::go!` — spawn coroutines
- `may::coroutine::yield_now()` / `co_yield` — cooperative yielding
- `may::sync::spsc` — coroutine-to-coroutine channels
- `may::queue::mpsc::Queue` — shared request queue
- `may::net::TcpStream` — may-aware TCP
- `WaitIo` / `WaitIoWaker` — epoll-based async I/O
- `may::timer::sleep` — timeouts

If a dependency requires tokio, find a may-compatible alternative or vendor / fork it. There is no exception.

### 2. Reference implementation: may_postgres

**`../may_postgres/` is the canonical reference for how to build a client like this.** When designing the connection layer, request-response pipeline, or may primitive usage, consult may_postgres first.

Specifically, mirror these patterns from may_postgres:
- Single `go!` coroutine running an epoll loop sharing one TCP socket
- mpsc `Queue<Request>` for sending requests from multiple coroutines
- spsc `Receiver<Response>` per-request for response dispatch
- Monotonically increasing tags for request-response matching
- Non-blocking read/write with `BytesMut` buffers
- `WaitIoWaker` to wake the connection loop when new requests arrive
- `may::run` / `may::go` for test setup (never `#[tokio::test]`)

Read [`docs/02-may_postgres_comparison.md`](./docs/02-may_postgres_comparison.md) for the detailed comparison and [`../may_postgres/`](../may_postgres/) for the actual code to follow.

### 3. Follow Rust conventions

- `snake_case` for fns / modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Group imports: std, external crates, local modules. Prefer explicit imports.
- `Result<T, E>` + `?` over `panic!` in library paths.
- No `unwrap()` / `expect()` in production code (`clippy::unwrap_used` is deny).

### 4. Modular architecture (target state)

The current code is a single crate with flat modules under `src/`. The planned architecture (documented in `docs/08-module-structure.md`) splits into a workspace of 6 crates:

```
base (no may) → codec (no may) → protocol (may) → connection (may+epoll) → client (assembles all) → may-redis (umbrella re-exports)
```

When modifying code, think in terms of module boundaries. Base should have zero may dependency. Codec should have zero may dependency. Only protocol, connection, and client may depend on may.

### 5. RESP2 only (v1 scope)

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
