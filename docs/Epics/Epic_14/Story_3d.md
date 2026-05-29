# Story 14.3d — URL parsing unit tests

**Objective:** Comprehensive unit tests for the full `rediss://` URL parsing flow.

**Epic:** 14 — TLS and mTLS Support (Sub-story 4/4 of Story 3)

**Dependencies:** Story 14.3c (connect_url rediss:// wiring)

## Deliverable

Unit tests for `connect_url()` with `rediss://` scheme covering all parameter combinations.

## Tasks

1. Patch `src/client/client.rs` — add unit tests in `#[cfg(test)] mod tests`:
   - `test_url_parse_rediss_basic` — `rediss://localhost:6380?system_certs=true` → TlsConfig built (may fail on connect, but parse should succeed)
   - `test_url_parse_rediss_mtls` — `rediss://host:6380?system_certs=true&client_cert=/tmp/cert.pem&client_key=/tmp/key.pem` → builds with ClientCerts
   - `test_url_parse_rediss_ca_path` — `rediss://host:6380?ca=/etc/ssl/ca.pem` → RustlsRootCerts::Pem
   - `test_url_parse_rediss_verify_false` — `rediss://host:6380?system_certs=true&verify=false` → verify_server=false
   - `test_url_parse_rediss_no_ca_fails` — `rediss://host:6380` without system_certs → Parse error
   - `test_url_parse_rediss_unknown_param_fails` — `rediss://host:6380?foobar=1` → Parse error
   - `test_url_parse_rediss_ipv6` — `rediss://[::1]:6380?system_certs=true` → host="::1", port=6380
   - `test_url_parse_rediss_special_chars` — `rediss://host:6380?ca=%2Fpath%20with%20spaces.pem` → decoded path
   - `test_url_parse_redis_plain` — `redis://localhost:6379` still works (backward compat)
   - `test_url_parse_double_prefix` — `rediss://rediss://host:6380` → Parse error
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — all URL tests pass
- `cargo fmt --all --check` passes
