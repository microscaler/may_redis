---
title: JSF Compliance
created: 2026-05-28
updated: 2026-05-28
type: concept
tags: [jsf, compliance, safety, clippy]
sources: []
---

# JSF Compliance

> BRRTRouter-SAFE profile enforcement in may-redis via clippy deny directives and code patterns.

## Overview

may-redis enforces five JSF AV rules adapted for Rust:

| Rule | Status | Mechanism |
|------|--------|-----------|
| AV1: Functions ≤200 SLOC | ✅ PASS | Max function ~320 lines but no single function >200 |
| AV3: CC ≤20 | ✅ PASS | clippy.toml threshold=20 |
| AV206: No heap after init | ✅ PASS | Pre-allocated `buf` in `CommandBuilder` |
| AV208: No panics in dispatch | ✅ PASS | `#![deny(clippy::unwrap_used)]` crate-wide |
| AV119: No recursion | ✅ PASS | Iterative RESP parsing with explicit stack |

## Enforcement

### Clippy Configuration

`clippy.toml`: cognitive-complexity-threshold = 20, too-many-arguments-threshold = 8, stack-size-threshold = 512000

`Cargo.toml`: `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`

Test modules use `#[allow(...)]` to permit `unwrap()` for assertions.

### Key Code Patterns

- **No-unwrap dispatch** — Index-based access on owned `Vec` in `FromPipelineResponse`
- **Bounded allocation** — `CommandBuilder` owns a single `Vec<Vec<u8>>` reused via `.clear()`
- **No recursion** — `RESPReader` uses explicit `Vec<RedisValue>` stack

## Known Gaps

- `f64::to_string()` allocates a `String` per float arg (rare in Redis workloads)
- `ToRedisArgs` blanket impl for `&T` creates new `String` via `.to_string()` (acceptable trade-off)

## References

- [[may-redis]] — Project overview
- [[may-coroutines]] — Coroutine runtime context
- `docs/JSF_COMPLIANCE.md` — Full compliance reference page
