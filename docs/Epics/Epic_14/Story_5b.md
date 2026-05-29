# Story 14.5b — version bounds validation

**Objective:** Validate `min_version <= max_version` in `TlsConfig::into_config()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 2/4 of Story 5)

**Dependencies:** Story 14.5a (TlsVersion::from_str exists)

## Deliverable

1. In `TlsConfig::into_config()`, add validation: reject `min_version > max_version`

## Tasks

1. Patch `src/tls/mod.rs` — in `TlsConfig::into_config()`:
   ```rust
   if self.min_version > self.max_version {
       return Err(TlsError::Config(format!(
           "min_version {:?} is greater than max_version {:?}",
           self.min_version, self.max_version,
       )));
   }
   ```
2. Add test: `test_tls_config_min_gt_max` — min=1.3, max=1.2 → error
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — version validation test passes
- `cargo fmt --all --check` passes
