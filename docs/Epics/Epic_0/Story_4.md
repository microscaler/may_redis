# Story 0.4 — Documentation structure

**Objective:** Organize the reference documentation and create the initial project README.

**Epic:** 0 — Scaffolding

**Dependencies:** None

**Source docs:** `docs/01-protocol-analysis.md`, `docs/02-may_postgres_comparison.md`, `docs/03-sesame-idam-redis-usage.md`

## Code Anchors

- `README.md` — project overview, architecture summary, getting started
- `docs/01-protocol-analysis.md` — reference: RESP wire format
- `docs/02-may_postgres_comparison.md` — reference: may_postgres patterns
- `docs/03-sesame-idam-redis-usage.md` — reference: Sesame-IDAM usage inventory
- `docs/Epics/Epic_0/` — epic definitions

## Tasks

1. Create `README.md` with:
   - Project title and description
   - Architecture diagram (mermaid) showing the 7-phase dependency chain
   - "How it works" section explaining may coroutines vs tokio
   - "Workspace structure" listing all crates
   - "Reference docs" linking to the 3 reference documents in `docs/`
   - "Epic plan" linking to `docs/Epics/`
2. Verify reference docs are in `docs/` (not inside epics)
3. Verify all epics are listed in README with links

## Verification

- `README.md` renders correctly and contains architecture mermaid diagram
- All reference docs are accessible from README
- All epics are listed in README with links
