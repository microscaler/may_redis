# Wiki Index

> Content catalog. Every wiki page listed under its type with a one-line summary.
> Last updated: 2026-05-28 | Total pages: 12

## Entities
- [[may-redis]] — Coroutine-native Redis client built on may runtime, single crate
- [[sesame-idam]] — Microscaler IDAM platform, 9 microservices using Redis for auth

## Concepts

## Comparisons

## Reference
- [[codebase-entry-points]] — Entry points into the may-redis codebase: src/lib.rs, modules, public API surface
- [[command-mapping]] — Commands trait method to RESP wire format mapping reference
- [[jsf-compliance]] — JSF-AV rule compliance: AV1, AV3, AV206, AV208, AV119, AV148/209 enforced via clippy

## Topics
- [[redis-command-coverage]] — 3-layer audit: Redis canonical vs may-redis vs sesame-idam (24% coverage, 100% sesame-idam fit)
- [[may-coroutines]] — Stackful coroutine runtime for Redis client I/O, no async/await
- [[resp-protocol]] — RESP2 wire format encoding/decoding, single crate module structure
- [[redis-command-expansion]] — Epic 7: systematic expansion of 22 to ~80 Redis commands across 7 stories
- [[may-redis-epic-7-story-1]] — String Extension commands: 13 commands added (DECR, DECRBY, SETNX, MGET, MSET, MSETNX, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, BITCOUNT_RANGE), 34 total command tests, clippy --lib clean
- [[connection-loop-pitfalls]] — Pitfalls in the may-coroutine epoll connection loop: event priorities, non-blocking I/O, response dispatch ordering
- [[may-coroutine-pattern]] — May coroutine patterns: go!, yield_now, spsc channels, WaitIo/WaitIoWaker, mpsc Queue, monotonically increasing tags
- [[module-structure]] — Target modular workspace architecture: base → codec → protocol → connection → client → may-redis umbrella re-exports
- [[sesame-idam-integration]] — Sesame-IDAM Redis usage: 11 canonical commands, 5 modules, command frequency analysis
- [[code-review-2026-05-28]] — Full codebase expert review: APPROVE with conditions (2 must-fix: worker thread blocking, missing safety comments)

## Queries
