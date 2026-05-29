# Story 14.4d — ssrf=true URL param

**Objective:** Add `ssrf=true` query parameter support in `connect_url()` for `rediss://` URLs.

**Epic:** 14 — TLS and mTLS Support (Sub-story 4/4 of Story 4)

**Dependencies:** Story 14.4c (RedisClient::connect_tls_with_ssrf exists)

## Deliverable

1. In `build_tls_config()` — parse `ssrf=true` → return SSRF config from params
2. In `connect_url()` — when ssrf param detected, call `connect_tls_with_ssrf()` instead of `connect_tls()`

## Tasks

1. Patch `src/client/client.rs` — in `connect_url()` rediss:// branch:
   ```rust
   let params = parse_tls_query_params(query)?;
   let ssrf_enabled = params.get("ssrf").map(|v| v == "true").unwrap_or(false);
   
   if ssrf_enabled {
       let config = build_tls_config(host, &params, port)?;
       let ssrf_config = crate::connection::SsrfConfig::default();
       Self::connect_tls_with_ssrf(host, port, &config, DEFAULT_EXECUTE_TIMEOUT, ssrf_config)
           .map_err(|e| RedisError::Parse(format!("TLS+SSRF connection failed: {e}")))
   } else {
       let config = build_tls_config(host, &params, port)?;
       Self::connect_tls(host, port, &config, DEFAULT_EXECUTE_TIMEOUT)
           .map_err(|e| RedisError::Parse(format!("TLS connection failed: {e}")))
   }
   ```
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
