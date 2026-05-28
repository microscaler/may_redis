# Story 8.4 — Remove Unused Dependencies

**Objective:** Remove `serde` and `serde_json` from `Cargo.toml`. These are declared as direct dependencies but are never imported or used anywhere in the codebase. They add ~2 seconds to compile time with zero functional benefit.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1, 8.2, 8.3 (all stories complete, zero test regressions).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #4, low severity but compile-time impact), `Cargo.toml`

## Functional Requirements

1. Remove `serde` from `[dependencies]` in `Cargo.toml`
2. Remove `serde_json` from `[dependencies]` in `Cargo.toml`
3. Verify no file in the codebase imports `serde` or `serde_json` (search `^use serde` and `^use serde_json`)
4. Verify `cargo build --workspace` still succeeds after removal
5. Verify `cargo test --lib` still passes after removal

## Non-Functional Requirements

1. **Verify no transitive dependency conflict** — `serde`/`serde_json` may be pulled in transitively by other crates (e.g., `may` or `bytes`). The removal must only strip the *direct* dependency. If another crate needs `serde`, it should declare it as its own dependency.
2. **Verify dev-dependency safety** — If either appears in `[dev-dependencies]` too, remove from there as well.
3. **Cargo.lock cleanup** — Run `cargo update` to update the lock file and verify `serde`/`serde_json` are no longer in the dependency tree (check `cargo tree`).

## Code Anchors

- `Cargo.toml` — `[dependencies]` section

## Tasks

1. Audit: `grep -r "use serde\|use serde_json" src/ tests/` — confirm zero results
2. Audit: `cargo tree -i serde` and `cargo tree -i serde_json` — check if transitively required
3. Remove `serde` from `[dependencies]` in `Cargo.toml`
4. Remove `serde_json` from `[dependencies]` in `Cargo.toml`
5. Run `cargo build --workspace` — verify clean build
6. Run `cargo test --lib` — verify all tests still pass
7. Run `cargo clippy --lib --tests --all-features -- -D warnings` — verify zero warnings
8. Run `cargo tree -i serde` — confirm no longer a transitive dep (optional, document if still present transitively)
9. Update `Cargo.lock` via `cargo update`

## Verification Checklist

- [ ] `grep -r "use serde\|use serde_json" src/ tests/` returns zero matches
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --lib` — all 262 tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `serde` and `serde_json` no longer in `Cargo.toml` dependencies
- [ ] If `serde`/`serde_json` are still in `cargo tree`, document as transitive (expected if any dep pulls them in)
