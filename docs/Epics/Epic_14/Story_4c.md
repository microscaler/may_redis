# Story 14.4c — RedisClient::connect_tls_with_ssrf

**Objective:** Add `RedisClient::connect_tls_with_ssrf()` method that chains SSRF → TCP → TLS → Connection.

**Epic:** 14 — TLS and mTLS Support (Sub-story 3/4 of Story 4)

**Dependencies:** Story 14.4b (from_tls_stream_with_ssrf exists)

## Deliverable

1. `RedisClient::connect_tls_with_ssrf(host, port, tls_config, timeout, ssrf_config)` → RedisClient

## Tasks

1. Patch `src/client/client.rs` — add `connect_tls_with_ssrf()` method:
   ```rust
   #[cfg(feature = "tls")]
   pub fn connect_tls_with_ssrf(
       host: &str,
       port: u16,
       config: &tls::TlsConfig,
       timeout: Duration,
       ssrf_config: crate::connection::SsrfConfig,
   ) -> Result<Self, RedisError> {
       let connection = crate::connection::Connection::connect_tls_with_ssrf(
           host, port, config, timeout, ssrf_config,
       ).map_err(|e| RedisError::Parse(format!("TLS connection failed: {e}")))?;
       Ok(Self {
           inner: Arc::new(InnerClient {
               connection,
               default_timeout: timeout,
               command_policy: crate::protocol::builder::CommandPolicy::AllowAll,
           }),
       })
   }
   ```
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
