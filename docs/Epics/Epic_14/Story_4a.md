# Story 14.4a — Connection::connect_tls_with_ssrf

**Objective:** Add `Connection::connect_tls_with_ssrf()` — SSRF check before TLS handshake.

**Epic:** 14 — TLS and mTLS Support (Sub-story 1/4 of Story 4)

**Dependencies:** Story 14.3c (connect_url rediss:// wiring)

## Deliverable

1. `Connection::connect_tls_with_ssrf(host, port, tls_config, timeout, ssrf_config)` — chains SSRF → TCP → TLS → Connection

## Tasks

1. Patch `src/connection/connection.rs` — add `connect_tls_with_ssrf()`:
   ```rust
   #[cfg(feature = "tls")]
   pub fn connect_tls_with_ssrf(
       host: &str,
       port: u16,
       tls_config: &tls::TlsConfig,
       timeout: std::time::Duration,
       ssrf_config: tcp::SsrfConfig,
   ) -> Result<Self, ConnectionError> {
       let stream = tcp::TcpConnector::connect_with_ssrf_check(
           host, port, timeout, ssrf_config,
       )?;
       let tls_stream = tls::TlsConnector::handshake(stream, tls_config, timeout)
           .map_err(|e| ConnectionError::Tls(e.to_string()))?;
       Self::from_tls_stream_with_ssrf(tls_stream, ssrf_config)
   }
   ```
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
