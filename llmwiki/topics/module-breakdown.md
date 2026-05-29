---
title: Module Breakdown Audit
created: 2026-05-29
updated: 2026-05-29
type: concept
tags: [architecture, coroutine, pipeline, testing, lint, coverage]
sources: [raw/articles/module-breakdown-audit-2026-05-29.md]
---

# Module Breakdown Audit

## What it is

Comprehensive audit of may-redis's single-crate module structure, identifying files exceeding the 350-line threshold and proposing domain-based splits.

## Decision: Split into domain sub-modules

The `Commands` trait in `protocol/commands.rs` (1,988 lines, 167 methods) is the primary candidate for reorganization. The split uses a directory-per-domain approach where each data type gets its own file:

- `commands/strings.rs` — GET, SET, SETEX, MGET, MSET, etc.
- `commands/hashes.rs` — HSET, HGET, HDEL, HGETALL, etc.
- `commands/sets.rs` — SADD, SISMEMBER, SMEMBERS, etc.
- `commands/lists.rs` — LPUSH, RPUSH, LPOP, RPOP, etc.
- `commands/sorted_sets.rs` — ZADD, ZREM, ZRANGE, ZRANK, etc.
- `commands/pubsub.rs` — SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, etc.
- `commands/transactions.rs` — MULTI, EXEC, DISCARD, WATCH, etc.
- `commands/admin.rs` — PING, AUTH, FLUSHDB, INFO, CONFIG, etc.

Each sub-module defines a small trait (e.g. `HashCommands`) that the main `Commands` trait extends. The blanket impl chains through all sub-traits.

## File size targets

After splits:
- 23 files -> ~36 files (more files, each smaller)
- Production files: 18 -> ~24 (excludes test-extraction-only splits)
- Max single file: 1,988 -> ~564 (protocol/builder.rs, no natural split point)
- Production files >350 lines: 8 -> 1

## Test extraction pattern

All `#[cfg(test)] mod tests { ... }` blocks extracted to separate `_tests.rs` files. This keeps production files lean without losing test coverage. The test files use `use super::*;` and `#[path = "..."]` declarations in `mod.rs`.

## Related

- [[module-structure]] — Target modular workspace architecture (base → codec → protocol → connection → client)
- [[may-coroutine-pattern]] — May coroutine runtime patterns
- [[resp-protocol]] — RESP2 wire format
