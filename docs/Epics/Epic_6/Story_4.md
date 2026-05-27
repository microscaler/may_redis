# Story 6.4 — Migration guide documentation

**Objective:** Document migration from `redis` crate to `may-redis`.

**Epic:** 6 — Integration & Migration

**Dependencies:** Story 6.3

**Status:** COMPLETE

**Source docs:** `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `docs/09-migration-guide.md` — migration guide

## Tasks

- [x] Create migration guide document
- [x] Document API surface parity (Commands trait methods)
- [x] Document connection pattern differences (coroutine vs blocking)
- [x] Document type mapping (RedisValue enum variants)
- [x] Document error handling differences (Result<T, RedisError>)
- [x] Document FromRedisValue type constraints
- [x] Document pipeline API differences
- [x] Add known differences section
- [x] Add verification checklist

## Verification

- `docs/09-migration-guide.md` exists and is comprehensive
