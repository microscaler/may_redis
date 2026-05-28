# Epic 9 — JSF-AV Compliance Hardening

**Objective:** Harden may-redis against the five critical JSF-AV rule categories identified in the BRRTRouter JSF analysis: bounded complexity, allocation discipline, no-panic dispatch, non-recursive control flow, and explicit types. Transform the codebase from "mostly compliant" to "enforced compliant."

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 0 through Epic 8 must all pass (`cargo test --lib` zero failures, clippy zero warnings).

**Source docs:**
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — Full JSF AV rules adapted to Rust, "BRRTRouter-SAFE" profile
- `BRRTRouter/docs/JSF/JSF_AUDIT_OPINION.md` — Expert assessment confirming BRRTRouter-SAFE profile
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — JSF compliance summary and performance validation
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — Allocation inventory and per-module analysis

## Rationale

BRRTRouter's JSF analysis identified five meaningful JSF principles that translate to Rust:

| JSF Rule | BRRTRouter Equivalent | may-redis Status |
|----------|----------------------|------------------|
| AV1: Functions ≤200 SLOC | Small, modular functions | ✅ PASS (max ~320 lines in commands.rs, but no single function >200) |
| AV3: Cyclomatic complexity ≤20 | Simple match/if branches | ✅ PASS |
| AV206: No heap after init | No Vec::new/format! in hot path | ⚠️ PARTIAL — `Vec::new()` in `to_args.rs` (14 instances), `builder.rs` (2 instances) |
| AV208: No panics in dispatch | All panics in tests; unwrap in pipeline | ⚠️ PARTIAL — `unwrap()` in production `FromPipelineResponse` impl |
| AV119: No recursion | Iterative only | ✅ PASS |
| AV148/209: Explicit types | Full type safety | ✅ PASS |

**Gap analysis:** Two areas need work:
1. **Allocation discipline** — `Vec::new()` in `to_args.rs` and `builder.rs` for every command argument encoding
2. **Dispatch safety** — `unwrap()` in production `FromPipelineResponse` pipeline deserialization

## Crate Architecture

Work spans three modules:

- `src/core/to_args.rs` — Replace `Vec::new()` with `SmallVec`-like or pre-allocated buffers for command argument encoding
- `src/protocol/builder.rs` — Replace `Vec::new()` with pre-allocated buffers for command encoding
- `src/client/pipeline.rs` — Replace `unwrap()` in `FromPipelineResponse` impls with proper error propagation

## Implementation Order

1. **Story 1: No-panic pipeline deserialization** — Safest entry point, no allocation changes, high impact on safety
2. **Story 2: Bounded allocation in builder** — Lower impact change (single file), introduces pre-allocation pattern
3. **Story 3: Bounded allocation in to_args** — Most impactful change (14 files), requires design decision on buffer strategy
4. **Story 4: Roundtrip invariant tests** — Test-only story ensuring all RedisValue variants roundtrip correctly
5. **Story 5: JSF lint profile** — Add clippy configuration or CI gate for complexity thresholds
6. **Story 6: JSF compliance documentation** — Create compliance reference page for the project

## Non-Functional Requirements

1. **Zero new dependencies** — No crates added to Cargo.toml. Any "smallvec-like" behavior must be implemented with standard library.
2. **RESP2 only** — All changes handle RESP2 wire format. RESP3 types are out of scope.
3. **Zero may dependency in core** — `to_args.rs`, `builder.rs` have no `may` imports.
4. **Backwards compatible** — Existing `ToRedisArgs` and `FromPipelineResponse` APIs unchanged; only error behavior improves.
5. **Test coverage** — Every change must have unit tests verifying the new behavior.
6. **No allocation regressions** — If pre-allocation increases memory for tiny commands, document the trade-off.

## Risks

- **SmallVec replacement** — Without `smallvec` crate, implementing bounded inline buffers requires manual array management. Must be careful with fixed-size arrays vs dynamic allocation.
- **to_args.rs API change** — The `write_redis_args(&self, buf: &mut Vec<Vec<u8>>)` signature takes `Vec`. Changing this to accept a generic buffer trait is a breaking API change. Must keep the signature and just change the internal allocation pattern.
- **Pipeline unwrap() semantics** — Current `unwrap()` panics on malformed server responses. Proper error handling would return `RedisError::Parse`. This is a behavior change but makes the client more resilient.
