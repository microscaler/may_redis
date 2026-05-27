# ADR 001 — Single Crate Module Structure

- **Status**: proposed
- **Date**: 2026-05-27
- **Deciders**: casibbald
- **Related**: AGENTS.md, docs/Epics/Epic_0/Story_2.md, docs/08-module-structure.md

## Context

When we designed the modular workspace split (docs/08-module-structure.md, Epic_0/Story_2.md), we modeled it after a large Rust workspace pattern: 6 crates with strict bottom-up dependencies (`base -> codec -> protocol -> connection -> client -> umbrella`). The rationale was test isolation (pure unit tests in base/codec, integration tests in connection/client) and feature flag granularity.

The current codebase is 3,747 lines across 21 source files in 6 crates:

```
crates/
├── base       5 files  ~596 LOC
├── client     4 files  ~985 LOC
├── codec      4 files  ~742 LOC
├── connection 4 files  ~646 LOC
├── may-redis  1 file   ~55 LOC
└── protocol   3 files  ~392 LOC
```

Each crate has its own `Cargo.toml`, `src/lib.rs`, and module files. The workspace root `Cargo.toml` just defines package membership and dependency wiring.

## Problems

### 1. Cargo.toml bloat relative to code

6 separate `Cargo.toml` files for 3,747 lines of code. The workspace root `Cargo.toml` is mostly dependency and lint configuration that could live in a single manifest. The per-crate `Cargo.toml` files are thin shells — they just declare the crate name, version, and a few path dependencies.

### 2. `crate_name::module` import noise

In a multi-crate workspace, imports cross crate boundaries:

```rust
use base::RedisValue;
use base::RedisError;
use base::FromRedisValue;
use base::ToRedisArgs;
use codec::RESPWriter;
use codec::RESPReader;
use protocol::builder::CommandBuilder;
use protocol::commands::Commands;
use connection::Connection;
use connection::Request;
```

In a single crate with modules, these become `base::`, `codec::`, `protocol::`, `connection::` — but they're module paths, not crate paths. The import syntax is identical, but the mental model is simpler: everything is in one binary/library.

### 3. Feature flag complexity

The planned feature flags (`base`, `codec`, `protocol`, `connection`, `client`, `pool`) require conditional compilation in every crate that depends on them. With 6 crates, a feature change ripples through the dependency graph. A single crate would need no feature flags for module selection — `base`, `codec`, `protocol`, `connection`, `client` would always be present because they're just modules, not separate crates.

### 4. Compile time is not meaningfully improved

With 6 crates, `cargo build` compiles each crate as a separate library target. In practice, most source files are small and the total compile time for 3,747 lines is measured in seconds either way. The marginal compile time improvement from splitting doesn't justify the maintenance overhead.

### 5. No publish target

This library is consumed only from sibling repos (sesame-idam, etc.) via path dependencies in Cargo.toml. There is no intention to publish to crates.io. A published library benefits from semantic versioning per-crate; an internal path dependency does not.

### 6. The "test isolation" rationale doesn't hold

docs/10-test-strategy.md argues that `base` and `codec` should be testable without any may/runtime dependency. This is achievable in a single crate too — just use `#[cfg(test)]` or a `feature = "unit-tests"` guard. The tests don't need a separate crate; they need a `#[cfg(test)]` module or a separate `tests/` directory.

## Options considered

### Option A: 6-crate workspace (current)

**Pros**: Strict compile-time dependency enforcement, per-crate test isolation, granular feature flags.
**Cons**: Bloat, import noise, no publish benefit, marginal compile savings, maintenance overhead.

### Option B: Single crate with module folders (proposed)

**Pros**: Single `Cargo.toml`, no cross-crate import noise, no feature flag complexity, trivial refactoring for tests (use `#[cfg(test)]`), mirrors the shape of small established crates like `regex`, `clap` (before v4 workspace), `reqwest` (before workspace split).
**Cons**: Loss of compile-time dependency enforcement between modules (cosmetic in Rust — `pub` module boundaries are still enforced by the compiler).

### Option C: Two-crate split (umbrella + main)

A middle ground: one `may-redis` crate with modules, and one `may-redis-testing` crate for `InMemoryClient`. Minimal gain over single crate; adds back the very complexity we're trying to avoid.

## Decision

**Choose Option B: single crate with module folders.**

The source tree becomes:

```
may_redis/
├── Cargo.toml           # Single manifest: name = "may-redis"
├── README.md
├── AGENTS.md
└── src/
    ├── lib.rs           # pub mod base; codec; protocol; connection; client;
    ├── base/
    │   ├── mod.rs       # pub mod redis_value; redis_error; from_redis_value; to_redis_args;
    │   ├── redis_value.rs
    │   ├── redis_error.rs
    │   ├── from_redis_value.rs
    │   └── to_redis_args.rs
    ├── codec/
    │   ├── mod.rs       # pub mod writer; reader; roundtrip;
    │   ├── writer.rs
    │   ├── reader.rs
    │   └── roundtrip.rs
    ├── protocol/
    │   ├── mod.rs       # pub mod builder; commands;
    │   ├── builder.rs
    │   └── commands.rs
    ├── connection/
    │   ├── mod.rs       # pub mod connection; tcp; epoll;
    │   ├── connection.rs
    │   ├── tcp.rs
    │   └── epoll.rs
    └── client/
        ├── mod.rs       # pub mod client; pipeline; in_memory;
        ├── client.rs
        ├── pipeline.rs
        └── in_memory.rs
```

Module paths stay the same (`base::`, `codec::`, `protocol::`, `connection::`, `client::`). Import syntax doesn't change. The only difference is everything is under one `Cargo.toml` manifest.

### What changes

1. Delete `crates/` directory entirely.
2. Create `src/base/`, `src/codec/`, `src/protocol/`, `src/connection/`, `src/client/` directories.
3. Move each source file into the corresponding module directory.
4. Create `mod.rs` in each module directory with `pub mod` declarations.
5. Update `src/lib.rs` to declare the 5 modules.
6. Consolidate all 6 `Cargo.toml` files into one root `Cargo.toml`.
7. Update `docs/` references that point to `crates/*/` paths.
8. Tests: `InMemoryClient` stays in `src/client/in_memory.rs` behind `#[cfg(feature = "test")]`. Other tests stay as `#[cfg(test)]` modules within their source files (no change needed).

### What stays the same

- Module names (`base`, `codec`, `protocol`, `connection`, `client`) — import paths unchanged.
- Every type, function, and trait name — zero refactoring of business logic.
- The connection loop algorithm, epoll pattern, spsc/mpsc channels, may runtime usage.
- Test strategy — `#[cfg(test)]` modules inside source files, integration tests in `#[cfg(test)]` at the bottom of source files.
- The 7 epics and 35 stories in `docs/Epics/` — story scope and verification criteria unchanged. Only story code anchors shift from `crates/*/src/` to `src/*/`.

### Risks

- **Epic stories already written** reference `crates/*/src/` paths. These need updating in docs (low effort, mechanical).
- **Feature flag granularity** is lost. But since there are no feature flags planned for v1 (all modules are always compiled), this is not a real loss.
- **Future growth** — if the library grows to 50K+ lines across multiple independent feature areas, a workspace split might make sense. But at 3.7K lines, there is no pressure to split.

## Consequences

This decision moves us from a workspace that models a large library to a structure that reflects the actual size of the project. The mental model changes from "here are 6 crates I need to reason about" to "here is one library with 5 modules."

The codebase becomes easier to navigate for new contributors (one `src/` tree, no `cd crates/`), easier to modify (no cross-crate import changes needed), and the `docs/` references become simpler (no `crates/` path prefix).
