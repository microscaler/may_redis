# Story 14.3c — connect_url rediss:// wiring

**Objective:** Wire `rediss://` URLs in `connect_url()` to call `parse_tls_query_params()` + `build_tls_config()` + `connect_tls()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 3/4 of Story 3)

**Dependencies:** Story 14.3b (build_tls_config compiles)

## Deliverable

1. Modify `connect_url()` `rediss://` branch:
   - Replace `"TLS is not yet supported"` with:
     - Parse query string after `?`
     - Call `parse_tls_query_params()` → HashMap
     - Call `build_tls_config(host, params, default_port(ConnectionScheme::Tls))` → TlsConfig
     - Call `Self::connect_tls(host, port, config, DEFAULT_EXECUTE_TIMEOUT)`
2. Remove `#[allow(dead_code)]` from `ConnectionScheme::Tls`
3. Handle `rediss://` double-prefix check before routing to TLS

## Tasks

1. Patch `src/client/client.rs` — in `connect_url()` `rediss://` branch:
   ```rust
   if let Some(rest) = url.strip_prefix("rediss://") {
       // Double-prefix check
       if rest.starts_with("rediss://") {
           return Err(RedisError::Parse("double URL scheme prefix (rediss://rediss://)".into()));
       }
       // Extract query string
       let (host_part, query) = rest.split_once('?').unwrap_or((rest, ""));
       // Parse host:port from host_part (same as redis://)
       let (host, port) = parse_host_port(host_part, default_port(ConnectionScheme::Tls))?;
       // Parse and build TLS config
       let params = parse_tls_query_params(query)?;
       let config = build_tls_config(host, &params, port)?;
       // Connect via TLS
       return Self::connect_tls(host, port, &config, DEFAULT_EXECUTE_TIMEOUT)
           .map_err(|e| RedisError::Parse(format!("TLS connection failed: {e}")));
   }
   ```
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
