# Story 10.1 — Lint Tightening (Cargo.toml)

**Objective:** Tighten `[lints.clippy]` in `Cargo.toml` — change the three `allow` rules (`missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc`) to `deny`.

**Epic:** 10 — Lint Tightening & Mandatory Rustdocs

**Dependencies:** None

**Status:** NOT STARTED

**Source docs:** `Cargo.toml`, `clippy.toml`, Epic 10 Story_0

## Code Anchors

- `Cargo.toml` — `[lints.clippy]` section

## Current State

```toml
[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
cast_precision_loss = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
module_name_repetitions = "allow"
struct_excessive_bools = "allow"
too_many_lines = "allow"
missing_errors_doc = "allow"        # <-- change to deny
missing_panics_doc = "allow"         # <-- change to deny
missing_safety_doc = "allow"         # <-- change to deny
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

## Tasks

1. Read `Cargo.toml` and locate the `[lints.clippy]` section
2. Change `missing_errors_doc = "allow"` to `missing_errors_doc = "deny"`
3. Change `missing_panics_doc = "allow"` to `missing_panics_doc = "deny"`
4. Change `missing_safety_doc = "allow"` to `missing_safety_doc = "deny"`
5. Verify the change with `cargo clippy --lib --tests --all-features` — expect ~41 warnings at this point (32 missing errors + 9 missing panics)
6. Do NOT fix the warnings yet — that is Story 10.2

## Verification

- `cargo clippy --lib --tests --all-features` — warns on all 32 `missing_errors_doc` and 9 `missing_panics_doc` items
- `cargo clippy --lib --tests --all-features` — zero warnings about `missing_safety_doc` (no `unsafe` public APIs exist)
- `cargo build --workspace` — still compiles (lint warnings, not errors yet)
