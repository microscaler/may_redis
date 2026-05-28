# Story 10.1 — Lint Tightening (Cargo.toml)

**Objective:** Tighten `[lints.clippy]` in `Cargo.toml` — change the three `allow` rules (`missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc`) to `deny`.

**Epic:** 10 — Lint Tightening & Mandatory Rustdocs

**Dependencies:** None

**Status:** COMPLETE

**Source docs:** `Cargo.toml`, `clippy.toml`, Epic 10 Story_0

**Commit:** `34e1037`

## Code Anchors

- `Cargo.toml` — `[lints.clippy]` section

## Changes Made

Changed three lint directives from `"allow"` to `"deny"`:

```toml
# Before:
missing_errors_doc = "allow"
missing_panics_doc = "allow"
missing_safety_doc = "allow"

# After:
missing_errors_doc = "deny"
missing_panics_doc = "deny"
missing_safety_doc = "deny"
```

## Verification

- `cargo clippy --lib --tests --all-features` — initially reported 41 errors (32 missing_errors_doc + 9 missing_panics_doc)
- Build still succeeds (lint errors only affect clippy, not compilation)
- All existing `allow(clippy::unwrap_used, ...)` and `#[allow(clippy::panic)]` annotations remain in place for test modules
