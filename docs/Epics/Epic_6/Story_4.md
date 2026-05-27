# Story 6.4 — Migration guide documentation

**Objective:** Create and verify the migration guide from the `redis` crate to `may-redis`.

**Epic:** 6 — Integration & Migration

**Dependencies:** Story 6.3

**Status:** MISSING — `docs/09-migration-guide.md` does not exist.

**Source docs:** `docs/03-sesame-idam-redis-usage.md`

## Migration Phases

| Phase | Action | Complexity |
|-------|--------|------------|
| 1 | Drop tokio-comp feature | Easy |
| 2 | Replace imports | Easy |
| 3 | Replace connection pattern | Medium |
| 4 | Replace query_async calls | Medium |
| 5 | Replace tokio::sync::Mutex | Medium |
| 6 | Fix test code | Easy |

## Code Anchors

- `docs/09-migration-guide.md` — **DOES NOT EXIST** — needs to be created
- `docs/03-sesame-idam-redis-usage.md` — Sesame-IDAM usage inventory

## Tasks

- [ ] Create `docs/09-migration-guide.md` from scratch
  - [ ] Phase 1: Drop tokio-comp feature — example diff
  - [ ] Phase 2: Replace imports (`redis::Client` → `may_redis::RedisClient`)
  - [ ] Phase 3: Replace connection pattern — `redis::Client::open()` → `may_redis::RedisClient::connect()`
  - [ ] Phase 4: Replace query_async calls — show `redis::Commands` vs `may_redis::Commands` side-by-side
  - [ ] Phase 5: Replace tokio::sync::Mutex — may uses coroutines, no mutex needed for single-client
  - [ ] Phase 6: Fix test code — replace `#[tokio::test]` with `may::run` + `may::go`
- [ ] Verify all code examples are syntactically correct (copy-paste testable)
- [ ] Add a "Verification checklist" section — exact steps to validate migration in sesame-idam
- [ ] Add "Known differences" section — what works differently between redis and may-redis
- [ ] Verify all Sesame-IDAM modules listed in `docs/03-sesame-idam-redis-usage.md` have a migration path documented

## Verification

- Migration guide code examples compile against actual may-redis crate
- No placeholders or "TBD" sections remain
- All Sesame-IDAM modules have a migration path documented
