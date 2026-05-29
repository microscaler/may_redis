# Story 14.3b — build_tls_config

**Objective:** Implement `build_tls_config()` — construct `TlsConfig` from parsed query parameters.

**Epic:** 14 — TLS and mTLS Support (Sub-story 2/4 of Story 3)

**Dependencies:** Story 14.3a (parse_tls_query_params compiles)

## Deliverable

1. `build_tls_config(host: &str, params: &HashMap<String, String>, default_port: u16) -> Result<TlsConfig, RedisError>`
2. Parses query params into TlsConfig fields:
   - `system_certs=true` → `RustlsRootCerts::WebPkiRoots`
   - `ca=/path/to/ca.pem` → `RustlsRootCerts::Pem(vec![PathBuf])`
   - `client_cert=/path` + `client_key=/path` → `ClientCerts::from_pem()`
   - `verify=false` → `verify_server: false`
   - `tls_min_version=1.3` → `TlsVersion::Tls13`
   - `tls_max_version=1.2` → `TlsVersion::Tls12`
   - `server_name=override` → SNI override
3. Error if neither `ca` nor `system_certs=true` is set
4. Unknown params return `Parse` error

## Tasks

1. Patch `src/client/client.rs` — add `build_tls_config()` function:
   - Extract `system_certs` → set root_certificates
   - Extract `ca` → set root_certificates to Pem([PathBuf])
   - Error if neither is set
   - Extract `verify` → set verify_server
   - Extract `tls_min_version` → parse with TlsVersion::from_str
   - Extract `tls_max_version` → parse with TlsVersion::from_str
   - Extract `server_name` → set server_name
   - Extract `client_cert` + `client_key` → read files, call ClientCerts::from_pem
2. Handle file I/O for client_cert/client_key paths
3. Add unit tests for build_tls_config:
   - `test_build_config_webpki` — system_certs=true builds WebPkiRoots
   - `test_build_config_ca_pem` — ca=/path builds Pem([path])
   - `test_build_config_no_ca_fails` — no ca or system_certs → Parse error
   - `test_build_config_verify_false` — verify=false
   - `test_build_config_version_bounds` — min=1.3, max=1.2 → error
4. Run `cargo build --features tls`
5. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — all build_config tests pass
- `cargo fmt --all --check` passes
