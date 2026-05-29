# Story 14.2b — ClientCerts from_pem

**Objective:** Add `ClientCerts::from_pem()` — parse PEM-encoded certificate chain and private key into DER.

**Epic:** 14 — TLS and mTLS Support (Sub-story 2/4 of Story 2)

**Dependencies:** Story 14.2a (ClientCerts struct exists)

## Deliverable

1. `ClientCerts::from_pem(cert_pem, key_pem) -> Result<Self, TlsError>` — parses PEM certs and key
2. Handles leaf + intermediate cert chain in cert_pem
3. Handles PKCS#8 and PKCS#1 private key formats

## Tasks

1. Patch `src/tls/mod.rs` — implement `ClientCerts::from_pem()`:
   - Parse certs: `rustls_pemfile::certs(&mut &cert_pem[..])` → collect to Vec<CertificateDer>
   - Convert to Vec<Vec<u8>> with `.to_vec()`
   - Parse key: `rustls_pemfile::private_key(&mut &key_pem[..])` → `PrivateKeyDer`
   - Extract DER: `key.secret_der()` → Vec<u8>
2. Add `TlsError::Config(String)` variant if not already present
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
