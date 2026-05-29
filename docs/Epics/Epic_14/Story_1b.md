# Story 14.1b — TlsConnector + polling handshake

**Objective:** Implement `TlsConnector::handshake()` — the polling TLS handshake loop using `may::coroutine::yield_now()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 2/4 of Story 1)

**Dependencies:** Story 14.1a (types compile)

## Deliverable

1. Implement `TlsConfig::into_config() -> Result<ClientConfig, TlsError>` — builds rustls ClientConfig from TlsConfig fields
2. Implement `RustlsRootCerts::to_root_store() -> Result<RootCertStore, TlsError>` — converts root certs to rustls store
3. Implement `SkipVerifier` — `ServerCertVerifier` trait impl (accepts any cert)
4. Implement `TlsConnector::handshake(stream, config, timeout) -> Result<TlsStream, TlsError>` — polling handshake loop

## Tasks

1. Patch `src/tls/mod.rs` — add `RustlsRootCerts::to_root_store()` implementation
   - WebPkiRoots: inject webpki_roots::TLS_SERVER_ROOTS
   - Pem: read files with rustls_pemfile::certs()
   - Der: parse DER with add_parsable_certificates()
2. Patch `src/tls/mod.rs` — implement SkipVerifier (dangerous cert verifier that accepts anything)
3. Patch `src/tls/mod.rs` — implement TlsConfig::into_config()
   - version bounds validation
   - crypto provider install_default
   - protocol version selection
   - build ClientConfig with root certs
   - with_no_client_auth()
4. Patch `src/tls/mod.rs` — implement TlsConnector::handshake()
   - ServerName from host
   - ClientConnection::new(config, server_name)
   - Polling loop: complete_io + yield_now + timeout check
   - Return TlsStream on success
5. Add unit tests: test_tls_version_from_str, test_tls_config_defaults, test_tls_version_ordering, test_tls_config_min_gt_max
6. Run `cargo build --features tls`
7. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — unit tests pass
- `cargo fmt --all --check` passes
