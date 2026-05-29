# Story 14.2d — Re-export + unit tests for mTLS

**Objective:** Re-export `ClientCerts` from crate root, add comprehensive unit tests for mTLS flow.

**Epic:** 14 — TLS and mTLS Support (Sub-story 4/4 of Story 2)

**Dependencies:** Story 14.2c (with_client_auth_cert wired)

## Deliverable

1. Re-export `ClientCerts` from `src/lib.rs`
2. Unit tests covering: from_pem success, from_der success, invalid PEM error, no client_certs builds standard TLS

## Tasks

1. Patch `src/lib.rs` — add `#[cfg(feature = "tls")] pub use tls::ClientCerts;`
2. Patch `src/tls/mod.rs` — add unit tests in `#[cfg(test)] mod tests`:
   - `test_tls_config_mtls_from_pem` — create ClientCerts from PEM, build config
   - `test_tls_config_mtls_from_der` — create from DER
   - `test_tls_config_mtls_invalid_pem` — invalid PEM → TlsError::Config
   - `test_tls_config_no_client_certs` — None builds standard TLS
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — all 4 mTLS tests pass
- `cargo fmt --all --check` passes
