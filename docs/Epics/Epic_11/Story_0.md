# Epic 11 — Code Review Remediation

**Objective:** Address all findings from the 2026-05-28 code review of may-redis. This epic groups all findings into actionable stories, prioritized by severity: Must-Fix, Should-Fix, and Nice-to-Have.

**Status:** IN PROGRESS

## Story Index

| Story | Title | Severity | Review Finding(s) | Status |
|-------|-------|----------|-------------------|--------|
| Story 0 | Epic Overview | — | — | ✅ COMPLETE |
| Story 1 | Replace `std::thread::sleep` with `may::timer::sleep` | MEDIUM | CL1 | |
| Story 2 | Add `// SAFETY:` comments to all `unsafe` blocks | MEDIUM | S1, C1, C2, C3 | |
| Story 3 | Fix `mget`/`mset`/`msetnx`/`sinter`/`sunion` API consistency | MEDIUM | P1 | |
| Story 4 | Remove redundant `impl Commands for RedisClient` bodies | LOW | P2 | |
| Story 5 | Remove dead `src/connection/epoll.rs` file | INFO | A3 | |
| Story 6 | Fix or exclude `examples/debug_redis.rs` clippy violation | LOW | S3 | |
| Story 7 | Add `// SAFETY:` comment for `Connection::drop` cancellation | LOW | C3, S2 | |
| Story 8 | Name magic buffer constants in `process_req` | LOW | C4 | |
| Story 9 | Document pub/sub commands require dedicated connection | INFO | P3 | |
| Story 10 | Document blocking command timeout considerations | INFO | P4 | |
| Story 11 | Add TLS and auth parsing to `connect_url` | INFO | CL4 | |
| Story 12 | Add upper-bound check for `usize` conversion on 32-bit platforms | LOW | T1 | |
| Story 13 | Add `FromRedisValue` impls for common integer types | INFO | T2 | |
| Story 14 | Final verification (clippy + tests + fmt) | — | — | |

## Findings Summary

This epic addresses 18 findings from the code review:

### Must-Fix (production blockers)
- **CL1** — `std::thread::sleep` blocks a may worker thread in timeout coroutine (MEDIUM)
- **C1/C2** — Missing `// SAFETY:` comments on 2 `unsafe` blocks (MEDIUM)

### Should-Fix (quality improvements)
- **P1** — `mget`/`mset`/`msetnx`/`sinter`/`sunion` don't take `&self` (MEDIUM)
- **P2** — Redundant `impl Commands for RedisClient` method bodies (LOW)
- **A3** — Dead `src/connection/epoll.rs` file (INFO)
- **S3** — `examples/debug_redis.rs` violates clippy deny rules (LOW)
- **C3** — Missing SAFETY comment for `Connection::drop` cancellation (LOW)
- **S2** — `Connection::drop` doesn't ensure in-flight request cleanup (LOW)
- **C4** — Magic buffer numbers in `process_req` (LOW)
- **T1** — No upper-bound check for `usize` conversion on 32-bit (LOW)

### Nice-to-Have (future work)
- **P3** — Pub/sub commands need documentation about dedicated connection (INFO)
- **P4** — Blocking commands (BLPOP/BRPOP) timeout documentation (INFO)
- **CL4** — `rediss://` URL and auth parsing (INFO)
- **T2** — Missing `FromRedisValue` impls for `u64`/`i32`/`u8`/`f64` (INFO)

## Dependency Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[CL1: may timer sleep]
    S0 --> S2[C1/C2: SAFETY comments]
    S1 --> S3[P1: mget/mset API]
    S2 --> S3
    S3 --> S4[P2: redundant impl]
    S4 --> S5[A3: dead epoll.rs]
    S5 --> S6[S3: example unwrap]
    S6 --> S7[C3/S2: drop SAFETY]
    S7 --> S8[C4: magic constants]
    S8 --> S9[P3: pub/sub docs]
    S9 --> S10[P4: blocking timeout docs]
    S10 --> S11[CL4: TLS URL parsing]
    S11 --> S12[T1: usize bound]
    S12 --> S13[T2: missing impls]
    S13 --> S14[Final verification]
```

Each story must pass `cargo clippy --lib --tests --all-features -- -D warnings` and `cargo fmt --all --check` before proceeding.
