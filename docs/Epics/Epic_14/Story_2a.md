# Story 14.2a — ClientCerts from_der

**Objective:** Add `ClientCerts` struct with DER-encoded certificate and key storage, plus `from_der()` constructor.

**Epic:** 14 — TLS and mTLS Support (Sub-story 1/4 of Story 2)

**Dependencies:** Story 14.1d (TlsConfig compiles)

## Deliverable

1. `ClientCerts` struct — two fields: `certificates: Vec<Vec<u8>>`, `private_key: Vec<u8>`
2. `ClientCerts::from_der()` — accepts DER-encoded cert chain and key
3. `Clone` impl on ClientCerts

## Tasks

1. Patch `src/tls/mod.rs` — add `ClientCerts` struct with `Clone`, `Debug` derives
2. Patch `src/tls/mod.rs` — implement `ClientCerts::from_der(certificates, private_key) -> Self`
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
