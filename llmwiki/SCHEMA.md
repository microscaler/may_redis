# may-redis LLM Wiki Schema

## Purpose
This wiki is a persistent, code-anchored knowledge layer between `docs/` and the Rust codebase.

## Source of Truth Order
1. Runtime behavior in `src/**` and `tests/**`
2. Existing prose docs in `docs/**`
3. This wiki (`llmwiki/**`) as reconciled synthesis

## Page Conventions
- Every substantive page includes:
  - **Status** (`verified`, `partially-verified`, `unverified`)
  - **Source docs** (`docs/...` links)
  - **Code anchors** (absolute repository paths)
  - **Gaps / drift** (doc claim vs code reality)
- Prefer explicit file paths and function names over high-level claims.
- Keep operational instructions executable and minimal.

## Operational Workflows
- **Ingest**: add/refresh entries from `docs/**` into `llmwiki/docs-catalog.md`, then reconcile with code.
- **Query**: answer from `llmwiki/index.md` + linked pages first, then verify in code when uncertain.
- **Build**: use `cargo test` per crate, `may::run` for integration tests, `InMemoryClient` for test isolation.

## Logging
- Append session updates to `llmwiki/log.md`.
- Keep entries chronological and append-only.
