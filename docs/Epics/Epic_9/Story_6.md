# Story 9.6 — JSF Compliance Documentation

**Objective:** Create a JSF compliance reference page for may-redis that documents the project's adherence to the JSF-AV rules, maps each rule to the corresponding code patterns, and provides a maintenance guide for future contributors.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 9 (all stories 1-5 complete).

**Source docs:**
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — BRRTRouter's compliance page (model)
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — Full JSF analysis
- `BRRTRouter/docs/JSF/JSF_AUDIT_OPINION.md` — Expert assessment

## The Goal

Create `docs/JSF_COMPLIANCE.md` in the may-redis repo that mirrors BRRTRouter's compliance page but is tailored to a Redis client rather than an HTTP router. This becomes the reference for:
1. New contributors understanding safety boundaries
2. Future JSF compliance reviews
3. Downstream consumers (sesame-idam) verifying may-redis safety guarantees

## Functional Requirements

1. Create `docs/JSF_COMPLIANCE.md` with the following sections:
   - **Executive Summary** — Overall JSF compliance status
   - **Rule Compliance Table** — Each JSF AV rule, BRRTRouter equivalent, may-redis status
   - **What We Enforce** — Clippy config, lint attributes, CI gates
   - **What We Don't Enforce (and why)** — E.g., no radix trie (not a router), small allocation footprint (Redis commands are small)
   - **Known Gaps** — Areas where JSF compliance could be improved
   - **References** — Links to BRRTRouter JSF docs, may-redis code anchors

2. Update `llmwiki/index.md` to reference the new JSF compliance page.

## Non-Functional Requirements

1. **No code changes** — documentation-only story.
2. **Consistent format** — mirrors BRRTRouter's JSF_COMPLIANCE.md structure.
3. **Accurate** — all claims must be verifiable against current code.

## Compliance Summary (anticipated)

| JSF Rule | may-redis Equivalent | Status | Notes |
|----------|---------------------|--------|-------|
| AV1: ≤200 SLOC functions | Small modular functions | ✅ PASS | No function >200 lines |
| AV3: CC ≤20 | Simple match/if | ✅ PASS | Code is modular |
| AV206: No heap after init | `Vec::new()` in builder/to_args | ⚠️ PARTIAL | Story 9.2/9.3 address |
| AV208: No panics | unwrap in pipeline | ⚠️ PARTIAL | Story 9.1 addresses |
| AV119: No recursion | Iterative only | ✅ PASS | All parsing is iterative |
| AV148/209: Explicit types | Full type safety | ✅ PASS | RedisValue, RedisError, etc. |

## Verification Checklist

- [ ] `docs/JSF_COMPLIANCE.md` created with all required sections
- [ ] Compliance table accurately reflects current code state
- [ ] Known gaps section is honest (not aspirational)
- [ ] `llmwiki/index.md` updated to reference JSF compliance page
- [ ] No code changes (documentation only)
