# Story 14.2c — with_client_auth_cert

**Objective:** Integrate `ClientCerts` into `TlsConfig::into_config()` — when `client_certs` is Some, build with `with_client_auth_cert()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 3/4 of Story 2)

**Dependencies:** Story 14.2b (ClientCerts::from_pem works)

## Deliverable

1. `TlsConfig::into_config()` — chain `with_client_auth_cert()` when `client_certs` is Some
2. Error handling: invalid private key format → `TlsError::Config`

## Tasks

1. Patch `src/tls/mod.rs` — in `TlsConfig::into_config()`:
   - After builder with root certs, add:
     ```rust
     let config = if let Some(certs) = self.client_certs {
         builder.with_client_auth_cert(
             certs.certificates.into_iter().map(CertificateDer::from).collect(),
             PrivateKeyDer::try_from(certs.private_key)
                 .map_err(|e| TlsError::Config(format!("invalid private key: {e}")))?,
         ).map_err(|e| TlsError::Config(format!("client auth error: {e}")))?
     } else {
         builder.with_no_client_auth()
     };
     ```
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
