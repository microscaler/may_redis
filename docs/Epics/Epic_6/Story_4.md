# Story 6.4 — Migration guide documentation

**Objective:** Finalize and verify the migration guide from `redis` crate to `may-redis`.

**Epic:** 6 — Integration & Migration

**Dependencies:** Story 6.3

**Source docs:** `docs/09-migration-guide.md`, `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `docs/09-migration-guide.md` — migration guide
- `docs/03-sesame-idam-redis-usage.md` — Sesame-IDAM usage inventory

## Migration Phases

| Phase | Action | Complexity |
|-------|--------|------------|
| 1 | Drop tokio-comp feature | Easy |
| 2 | Replace imports | Easy |
| 3 | Replace connection pattern | Medium |
| 4 | Replace query_async calls | Medium |
| 5 | Replace tokio::sync::Mutex | Medium |
| 6 | Fix test code | Easy |

## Tasks

1. Review and update `docs/09-migration-guide.md` against actual may-redis API
2. Verify all code examples in the migration guide are syntactically correct (copy-paste testable)
3. Add a "Verification checklist" section — exact steps to validate migration in sesame-idam
4. Add "Known differences" section — what works differently between redis and may-redis
5. Verify all Sesame-IDAM modules listed in `docs/03-sesame-idam-redis-usage.md` have a migration path documented

## Verification

- Migration guide code examples compile against actual may-redis crate
- No placeholders or "TBD" sections remain
- All Sesame-IDAM modules have a migration path documented
