# TLS and mTLS Support (Epic 14)

> TLS encryption and mutual TLS authentication for Redis connections, feature-gated behind `tls` Cargo feature.

## Overview

- **Status:** Stories 14.1-14.5 implemented, code compiles and tests pass
- **Feature flag:** `tls`
- **Crypto backend:** rustls 0.23 with `ring`
- **No `.await`, no `tokio`** — all I/O via may coroutines

## Implementation Stories

### Story 14.1 — TLS Foundation
- Cargo.toml: `rustls` + `webpki-roots` dependencies
- `TlsConfig` struct (root certs, client certs, server name, min/max versions, verify flag)
- `TlsConnector::handshake()` — polling-based TLS handshake with may coroutine yields
- `TlsError` enum (Config, HandshakeTimeout, Handshake, Verification, InvalidTlsVersion, ClientCertRequired)
- `TlsStream` — wraps rustls `ClientConnection` + TCP stream

### Story 14.2 — mTLS
- `RustlsRootCerts` enum: WebPkiRoots, Pem(Vec<PathBuf>), Der(Vec<Vec<u8>>)
- `ClientCerts::from_pem()` — PEM cert chain + private key parsing
- `ClientCerts::from_der()` — direct DER construction
- `into_config()` — builds rustls `ClientConfig` with mTLS cert

### Story 14.3 — URL Parsing
- `rediss://` URL scheme support
- Query params: `timeout`, `ca_cert`, `client_cert`, `client_key`, `verify_server`, `system_certs`, `server_name`, `tls_min_version`, `tls_max_version`
- Case-insensitive param names, URL-decoded values
- Unknown params return `Parse` error
- TLS version parsing ("1.2", "1.3") with min/max validation

### Story 14.4 — SSRF Protection for TLS
- `SsrfConfig::default()` — deny_private=true, deny_link_local=true, deny_loopback=false
- `connect_tls_with_ssrf()` — SSRF check runs before TCP connect
- `from_tls_stream_with_ssrf()` — stores SSRF config on TLS connection
- `RedisClient::connect_tls_with_ssrf()` — chains SSRF → TCP → TLS → Connection
- `rediss://` with `ssrf=true`/`ssrf=false` query parameter

### Story 14.5 — TLS Config Options
- `TlsVersion::parse()` — parse "1.2" or "1.3", return error for invalid
- `TlsVersion::to_supported()` — convert to `rustls::SupportedProtocolVersion`
- `into_config()` sets `with_min_protocol_version()` and `with_max_protocol_version()`
- Validation: `min_version <= max_version`, returns error if violated
- URL params: `tls_min_version=1.2|1.3`, `tls_max_version=1.2|1.3`

## Source Files

| File | Lines | Description |
|------|-------|-------------|
| `src/tls/mod.rs` | 19 | Module exports |
| `src/tls/config.rs` | 167 | TlsVersion, RustlsRootCerts, ClientCerts |
| `src/tls/connector.rs` | 257 | TlsError, TlsConfig, TlsConnector::handshake() |
| `src/tls/tls_stream.rs` | 71 | TlsStream wrapper (Read/Write impls) |
| `src/tls/tests.rs` | 215 | Unit tests for config and errors |
| `src/connection/connection_tls.rs` | 135 | connect_tls() and connect_tls_with_ssrf() |
| `src/client/client_url.rs` | 569 | URL parsing including rediss:// |
| `src/client/client_timeout.rs` | 166 | Timeout-aware execution |

## Test Coverage

- 21 tests in `tls/tests.rs` — TlsVersion, TlsConfig, ClientCerts, TlsError Display
- 19 tests in `client_url.rs` — URL decode, query params, basic connect
- 10 tests in `tcp_tests.rs` — ConnectionError Display, resolve, basic connect

**Known gaps:** SSRF deny-list (15 IP ranges), TlsConnector::handshake timeout path, execute_with_timeout cancellation, TlsStream constructors, 9 Connection struct methods — see `docs/Epics/Epic_14/TEST_GAP_ANALYSIS.md`

## Feature Flags

```toml
# Cargo.toml
rustls = { version = "0.23", optional = true, default-features = false, features = ["ring", "std", "tls12"] }
webpki-roots = { version = "0.26", optional = true }
rustls-pemfile = { version = "2", optional = true }
```

## API Surface

- `RedisClient::connect(url)` — plain TCP
- `RedisClient::connect_with_timeout(host, port, timeout)` — TCP with timeout
- `RedisClient::connect_tls(host, port, tls_config, timeout)` — TLS
- `RedisClient::connect_tls_with_ssrf(host, port, tls_config, timeout, ssrf_config)` — TLS with SSRF
- `connect_url(url)` — `redis://` or `rediss://` with query params
- `execute_with_timeout(cmd, timeout)` — timeout-aware command execution

## Design Decisions

1. **rustls + ring** — zero assembly dependencies, works on macOS/Linux/Windows
2. **No TLS handshake timeout separate from connect timeout** — same Duration used for both
3. **SSRF config stored on Connection** — same for TCP and TLS, accessible via getter
4. **Hash-tag extraction in CRC16** — Redis Cluster spec: `{tag}:suffix` hashes only "tag"
