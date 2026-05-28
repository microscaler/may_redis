# Wiki Index

> Content catalog. Every wiki page listed under its type with a one-line summary.
> Last updated: 2026-06-01 | Total pages: 10

## Entities
- [[may-redis]] — Coroutine-native Redis client built on may runtime, single crate
- [[sesame-idam]] — Microscaler IDAM platform, 9 microservices using Redis for auth

## Concepts
- [[redis-command-coverage]] — 3-layer audit: Redis canonical vs may-redis vs sesame-idam (24% coverage, 100% sesame-idam fit)
- [[may-coroutines]] — Stackful coroutine runtime for Redis client I/O, no async/await
- [[resp-protocol]] — RESP2 wire format encoding/decoding, single crate module structure
- [[redis-command-expansion]] — Epic 7: systematic expansion of 22 to ~80 Redis commands across 7 stories
- [[may-redis-epic-7-story-1]] — String Extension commands: 13 commands added (DECR, DECRBY, SETNX, MGET, MSET, MSETNX, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, BITCOUNT_RANGE), 34 total command tests, clippy --lib clean

## Reference
- [[codebase-entry-points]] — Entry points into the may-redis codebase: src/lib.rs, modules, public API surface
- [[command-mapping]] — Commands trait method to RESP wire format mapping reference

## Topics
- [[connection-loop-pitfalls]] — Pitfalls in the may-coroutine epoll connection loop: event priorities, non-blocking I/O, response dispatch ordering
- [[may-coroutine-pattern]] — May coroutine patterns: go!, yield_now, spsc channels, WaitIo/WaitIoWaker, mpsc Queue, monotonically increasing tags
- [[module-structure]] — Target modular workspace architecture: base → codec → protocol → connection → client → may-redis umbrella re-exports
- [[resp-protocol]] — RESP2 wire format: bulk strings, arrays, integers, errors, single crate encoding/decoding
- [[sesame-idam-integration]] — Sesame-IDAM Redis usage: 11 canonical commands, 5 modules, command frequency analysis

## Comparisons

## Queries
