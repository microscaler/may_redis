# Story 14.5d — Version unit tests

**Objective:** Comprehensive unit tests for TLS version parsing and bounds.

**Epic:** 14 — TLS and mTLS Support (Sub-story 4/4 of Story 5)

**Dependencies:** Story 14.5c (version params wired)

## Deliverable

Unit tests for TlsVersion parsing and TlsConfig version bounds.

## Tasks

1. Patch `src/tls/mod.rs` — add tests:
   - `test_tls_version_from_str_12` — "1.2" → Tls12
   - `test_tls_version_from_str_13` — "1.3" → Tls13
   - `test_tls_version_from_str_invalid` — "1.1" → TlsError::InvalidTlsVersion
   - `test_tls_version_from_str_empty` — "" → TlsError::InvalidTlsVersion
   - `test_tls_version_from_str_whitespace` — " 1.2 " → Tls12 (trimmed)
   - `test_tls_config_default_versions` — defaults are Tls12 min, Tls13 max
   - `test_tls_config_13_only` — min=1.3, max=1.3 builds 1.3-only config
   - `test_tls_config_min_gt_max_fails` — min=1.3, max=1.2 → error
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — all 8 version tests pass
- `cargo fmt --all --check` passes
