# Story 14.5c — URL version params

**Objective:** Wire `tls_min_version` and `tls_max_version` query params in `build_tls_config()` and `connect_url()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 3/4 of Story 5)

**Dependencies:** Story 14.5b (version bounds validation)

## Deliverable

1. In `build_tls_config()` — parse `tls_min_version` and `tls_max_version` query params, call `TlsVersion::from_str()`
2. Default to Tls12/Tls13 when not specified

## Tasks

1. Patch `src/client/client.rs` — in `build_tls_config()`:
   ```rust
   let min_ver = params.get("tls_min_version")
       .map(|s| TlsVersion::from_str(s))
       .transpose()?
       .unwrap_or(TlsVersion::Tls12);
   let max_ver = params.get("tls_max_version")
       .map(|s| TlsVersion::from_str(s))
       .transpose()?
       .unwrap_or(TlsVersion::Tls13);
   ```
2. Pass parsed versions into TlsConfig constructor
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
