# Story 11.6 — Fix or exclude `examples/debug_redis.rs` clippy violation

**Objective:** Fix the `examples/debug_redis.rs` file so it no longer violates clippy deny rules (uses `unwrap()`), OR exclude it from clippy checks via Cargo.toml.

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding S3, LOW)

**Finding:** S3 — `examples/debug_redis.rs` uses `unwrap()` which violates `clippy::unwrap_used` deny rules. This is not production code but it still fails clippy when run on the full workspace.

## Functional Requirements

- [ ] `examples/debug_redis.rs` must compile cleanly under `cargo clippy --lib --tests --all-features -- -D warnings`
- [ ] Either: fix the `unwrap()` calls with proper error handling (use `?`/`match`), OR exclude the example from clippy via `[[example]]` config in `Cargo.toml`

## Non-Functional Requirements

- [ ] The example should remain a useful debugging aid — its purpose is to verify TCP connectivity to Redis
- [ ] If fixed: use proper `Result` propagation with descriptive error messages
- [ ] If excluded: the exclusion must be explicit and documented in Cargo.toml

## Code Anchors

- `examples/debug_redis.rs` — The example file with `unwrap()` calls
- `Cargo.toml` — Where `[[example]]` config would go (if excluding)

## Decision Point

**Approach A (preferred):** Fix the example with proper error handling. Examples are part of the public API surface and should demonstrate best practices.

**Approach B (acceptable):** Exclude from clippy via:
```toml
[[example]]
name = "debug_redis"
test = false
doctest = false

[example.lints]
clippy.unwrap_used = "allow"
clippy.expect_used = "allow"
clippy.panic = "allow"
```

## Tasks

1. Audit all `unwrap()`/`expect()`/`panic!()` calls in the example
2. Choose approach A (fix) or B (exclude)
3. Implement the chosen fix
4. Verify clippy passes

## Verification

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Example specifically passes clippy

### Build

- [ ] `cargo build --example debug_redis` — compiles
- [ ] The example still runs correctly against a live Redis server

### Format

- [ ] `cargo fmt --all --check` — clean

### Expected Results

- `examples/debug_redis.rs` is clippy-clean
- Example remains functional as a debugging tool
- Zero impact on production code
