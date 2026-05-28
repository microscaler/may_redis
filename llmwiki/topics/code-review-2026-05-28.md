---
title: Code Review 2026-05-28
created: 2026-05-28
updated: 2026-05-28
type: summary
tags: [architecture, redis, coroutine, testing]
sources: [docs/code-review-2026-05-28.md]
---

# Code Review — 2026-05-28

> Full-codebase expert review of may-redis main branch.
> Result: **APPROVE with conditions** (2 must-fix items)

## Scope

- All 5 modules: [[may-redis]] core, codec, protocol, connection, client
- Build: ✅ clean | Tests: 313 pass, 28 ignored (no Redis) | Clippy: clean (lib+tests)
- Reviewed against [[may-coroutines]] patterns, [[resp-protocol]] correctness, Redis API fidelity

## Key Findings

### Must-Fix (Production Blockers)

1. **CL1 — Worker thread blocking in timeout** (`src/client/client.rs:103-106`)
   - `execute_with_timeout` spawns `go!(move || { std::thread::sleep(timeout); ... })`
   - `std::thread::sleep` blocks an entire may worker thread
   - Fix: replace with `may::timer::sleep` or cooperative sleep
   - Impact: under load with few workers, can starve the [[may-coroutines]] scheduler

2. **S1 — Missing `// SAFETY:` comments** (`src/connection/connection.rs`)
   - 3 `unsafe` blocks (nonblock_read, nonblock_write, Connection::drop) lack safety documentation
   - Code is correct (mirrors [[may-coroutine-pattern]] from may_postgres) but violates Rust best practice
   - Impact: maintenance risk if invariants aren't documented for future contributors

### Medium Findings

3. **P1 — Inconsistent associated functions** — `mget`, `mset`, `msetnx`, `sinter`, `sunion` don't take `&self`, breaking the ergonomic `client.command(...)` pattern
4. **A2 — Redundant allow attributes** — inner-module `#![allow(...)]` duplicates crate-level allows

### Low/Info Findings

5. Dead file `src/connection/epoll.rs` (unused)
6. Redundant `impl Commands for RedisClient` bodies (trait has defaults)
7. Pub/sub commands build correct wire format but connection loop lacks push-mode state machine
8. `examples/debug_redis.rs` uses `unwrap()` violating clippy deny rules
9. `FromRedisValue for usize` lacks upper-bound check for 32-bit platforms

## Architecture Assessment

| Module | Quality | Notes |
|--------|---------|-------|
| core/ | ★★★★★ | Clean types, no runtime dependency, comprehensive impls |
| codec/ | ★★★★★ | Hardened, configurable caps, strict CRLF, roundtrip tested |
| protocol/ | ★★★★☆ | 80+ commands, minor API consistency issue (P1) |
| connection/ | ★★★★☆ | Correct loop with documented pitfalls, needs safety comments |
| client/ | ★★★★☆ | Good API, timeout implementation needs may::timer (CL1) |

## Related Pages

- [[connection-loop-pitfalls]] — Bug 1 and Bug 2 documented and regression-tested
- [[jsf-compliance]] — 5/6 JSF-AV rules pass
- [[module-structure]] — current single-crate vs target workspace
- [[redis-command-coverage]] — 80+ commands, 100% sesame-idam coverage

## Performance Notes

- Pre-allocated 64KB buffers in connection loop ✅
- Zero-allocation integer formatting (itoa) ✅
- TCP_NODELAY on connect ✅
- No connection pooling (single connection per RedisClient) — feature gap for high-throughput use
- No automatic reconnection on connection drop — feature gap

## Full Report

See `docs/code-review-2026-05-28.md` for the complete audit with line-level citations.
