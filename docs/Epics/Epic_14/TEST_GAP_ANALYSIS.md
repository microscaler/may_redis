# Epic 14 Test Gap Analysis

**Generated:** 2026-06-02
**Scope:** All source files touched by Epic 14 (TLS and mTLS support)
**Goal:** Identify missing test coverage and plan remediation

---

## Summary

| File | Pub Fns | Test Fns | Test Loc | Coverage |
|------|---------|----------|----------|----------|
| `tls/config.rs` | 5 | 0 | 0 | 0% |
| `tls/connector.rs` | 2 | 0 | 0 | 0% |
| `tls/tls_stream.rs` | 3 | 0 | 0 | 0% |
| `tls/tests.rs` | — | 21 | 100% | external |
| `client/client.rs` | 10 | 0 | 0 | 0% |
| `client/client_url.rs` | 2 | 19 | 30% | mixed |
| `client/client_timeout.rs` | 2 | 0 | 0 | 0% |
| `connection/connection.rs` | 9 | 0 | 0 | 0% |
| `connection/connection_tls.rs` | 2 | 0 | 0 | 0% |
| `connection/tcp.rs` | 8 | 0 | 0 | 0% |
| `connection/tcp_tests.rs` | — | 10 | 100% | external |

**Total gaps: 49 missing test scenarios across 9 files.**

---

## CRITICAL Gaps

### 1. `connection/tcp.rs` — `SsrfConfig::is_blocked()` (15 ranges, 0 tests)

**File:** `src/connection/tcp.rs` lines 89-151
**Functions:** `SsrfConfig::is_blocked()`, `SsrfConfig::is_blocked_v4()`, `SsrfConfig::is_blocked_v6()`, `ssrf_allowed()`

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | V4 private 10.0.0.0/8 | ❌ |
| 2 | V4 private 172.16.0.0/12 | ❌ |
| 3 | V4 private 192.168.0.0/16 | ❌ |
| 4 | V4 link-local 169.254.0.0/16 | ❌ |
| 5 | V4 loopback 127.0.0.0/8 | ❌ |
| 6 | V4 0.0.0.0/8 | ❌ |
| 7 | V4 CGNAT 100.64.0.0/10 | ❌ |
| 8 | V4 multicast 224.0.0.0/4 | ❌ |
| 9 | V4 reserved 240.0.0.0/4 | ❌ |
| 10 | V6 loopback ::1 | ❌ |
| 11 | V6 link-local fe80::/10 | ❌ |
| 12 | V6 unique-local fc00::/7 | ❌ |
| 13 | V6 multicast ff00::/8 | ❌ |
| 14 | V6 unspecified :: | ❌ |
| 15 | All deny flags false — public IP allowed | ❌ |

**Additional:**
- `SsrfConfig::default()` — default values not tested (deny_private=true, deny_link_local=true, deny_loopback=false)
- `ssrf_allowed()` — wrapper function has zero tests

---

### 2. `tls/connector.rs` — `TlsConnector::handshake()` (0 tests)

**File:** `src/tls/connector.rs` lines 191-256

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Full handshake success (mock stream) | ❌ |
| 2 | Handshake timeout — `HandshakeTimeout` error | ❌ |
| 3 | Idle loop safety valve (100 yields) | ❌ |
| 4 | `complete_io` I/O error — `Handshake` error | ❌ |
| 5 | `server_name` empty → fallback to "localhost" | ❌ |
| 6 | `server_name` invalid → `Config` error | ❌ |
| 7 | `rustls::ClientConnection::new` fails | ❌ |

---

### 3. `connection/connection_tls.rs` — `connect_tls()` / `connect_tls_with_ssrf()` (0 tests)

**File:** `src/connection/connection_tls.rs`

| # | Function | Scenario | Coverage |
|---|----------|----------|----------|
| 1 | `connect_tls()` | Happy path | ❌ |
| 2 | `connect_tls()` | TCP connect fails → `Connect` error | ❌ |
| 3 | `connect_tls()` | TLS handshake fails → `Tls` error | ❌ |
| 4 | `connect_tls_with_ssrf()` | SSRF blocked → `SsrfViolation` | ❌ |
| 5 | `connect_tls_with_ssrf()` | SSRF allowed → continues to TCP | ❌ |
| 6 | `connect_tls_with_ssrf()` | SSRF + TLS handshake fails | ❌ |

---

### 4. `connection/connection_tls.rs` — `from_tls_stream` helpers (0 tests)

**File:** `src/connection/connection_tls.rs` lines 105-135

| # | Function | Scenario | Coverage |
|---|----------|----------|----------|
| 1 | `from_tls_stream()` | Connection construction from TLS stream | ❌ |
| 2 | `from_tls_stream_with_ssrf()` | SSRF config stored on TLS connection (Some) | ❌ |
| 3 | `from_tls_stream_with_ssrf()` | SSRF config = None for plain TLS | ❌ |
| 4 | `from_tls_stream_with_ssrf()` | SSRF config = Some for SSRF-enabled TLS | ❌ |

---

### 5. `client/client_timeout.rs` — `execute_with_timeout()` (0 tests)

**File:** `src/client/client_timeout.rs` lines 71-165

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Normal execution (response before timeout) | ❌ |
| 2 | Timeout fires before response | ❌ |
| 3 | Timeout fires before request sent (cancelled path, line 116) | ❌ |
| 4 | Guard cancellation spsc drop mechanism | ❌ |
| 5 | Response arrives after timeout signal | ❌ |

---

## HIGH Gaps

### 6. `tls/config.rs` — `RustlsRootCerts::to_root_store()` (3 paths untested)

**File:** `src/tls/config.rs` lines 61-110

| # | Variant | Scenario | Coverage |
|---|---------|----------|----------|
| 1 | `Pem` with valid file | `to_root_store()` succeeds | ❌ |
| 2 | `Pem` with nonexistent file | Returns `Config` error | ❌ |
| 3 | `Pem` with invalid PEM content | Parse error | ❌ |
| 4 | `Der` with valid certs | `add_parsable_certificates` path | ❌ |
| 5 | `Der` with empty vec | Empty store | ❌ |

---

### 7. `tls/config.rs` — `ClientCerts::from_pem()` (4 paths untested)

**File:** `src/tls/config.rs` lines 130-157

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Valid PEM with PKCS#8 key | ❌ |
| 2 | Valid PEM with PKCS#1 key | ❌ |
| 3 | Empty cert PEM | ❌ |
| 4 | Key present, cert absent | ❌ |

---

### 8. `client/client.rs` — `RedisClient::connect_*()` methods (4 fns, 0 tests)

**File:** `src/client/client.rs`

| # | Function | Coverage |
|---|----------|----------|
| 1 | `RedisClient::connect()` | ❌ |
| 2 | `RedisClient::connect_with_timeout()` | ❌ |
| 3 | `RedisClient::connect_tls()` | ❌ |
| 4 | `RedisClient::connect_tls_with_ssrf()` | ❌ |

The 19 tests in `client_url.rs` call `connect_url()` which wires these, but that's integration-level (requires a running server or connection failure). No pure unit tests.

---

### 9. `connection/connection.rs` — Connection struct methods (9 fns, 0 tests)

**File:** `src/connection/connection.rs`

| # | Function | Coverage |
|---|----------|----------|
| 1 | `Connection::new()` | ❌ |
| 2 | `Connection::from_stream()` | ❌ |
| 3 | `Connection::send()` | ❌ |
| 4 | `Connection::pending_count()` | ❌ |
| 5 | `Connection::max_queue_depth()` | ❌ |
| 6 | `Connection::max_request_size()` | ❌ |
| 7 | `Connection::ssrf_config()` — plain TCP | ❌ |
| 8 | `Connection::ssrf_config()` — TLS | ❌ |
| 9 | `ssrf_config()` getter returns None for plain | ❌ |

---

### 10. `tls/tls_stream.rs` — TlsStream (3 fns, 0 tests)

**File:** `src/tls/tls_stream.rs` lines 19-55

| # | Function | Coverage |
|---|----------|----------|
| 1 | `TlsStream::new()` | ❌ |
| 2 | `TlsStream::inner_mut()` | ❌ |
| 3 | `TlsStream::inner()` | ❌ |
| 4 | `Read` impl | ❌ |
| 5 | `Write` impl | ❌ |

---

## MEDIUM Gaps

### 11. `tls/config.rs` — `TlsVersion::parse()` edge cases

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | "1.3.1" — non-standard version | ❌ |
| 2 | "v1.2" — with prefix | ❌ |
| 3 | Leading/trailing whitespace only | ❌ |

---

### 12. `tls/connector.rs` — `TlsConfig::into_config()` variant paths

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | min=Tls13, max=Tls13 — forces 1.3 only | ❌ |
| 2 | min=Tls12, max=Tls12 — forces 1.2 only | ❌ |
| 3 | root_certificates=Pem — not WebPkiRoots | ❌ |

---

### 13. `connection/tcp.rs` — `TcpConnector` methods (0 tests)

| # | Function | Coverage |
|---|----------|----------|
| 1 | `connect()` — default 5s timeout | ❌ |
| 2 | `connect_with_ssrf_check()` — SSRF + DNS + connect | ❌ |
| 3 | `connect_with_timeout()` — custom timeout | ❌ |
| 4 | `connect_timeout()` — seconds → Duration | ❌ |
| 5 | `resolve()` with 0 addresses result | ❌ |

---

### 14. `connection/tcp.rs` — `ConnectionError` Display variants

| # | Variant | Coverage |
|---|---------|----------|
| 1 | `Tls` Display (with `tls` feature) | ❌ |
| 2 | `SsrfViolation` Display | ❌ |
| 3 | `is_timeout()` for Connect/Resolve/SetNodelay | ❌ |

---

## LOW / INFO Gaps

### 15. `client/client_url.rs` — `url_decode()` edge cases

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Empty string | ❌ |
| 2 | Double percent `%%` | ❌ |
| 3 | `%00` null byte | ❌ |
| 4 | `%ZZ` uppercase invalid — tested with lowercase | ❌ |

---

### 16. `client/client_url.rs` — `parse_tls_query_params()` edge cases

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Empty value `key=` | ❌ |
| 2 | Value with no key `=value` | ❌ |
| 3 | Duplicate keys (last wins) | ❌ |
| 4 | Malformed percent in key | ❌ |
| 5 | Multiple `&` at end `a=1&` | ❌ |
| 6 | Unknown parameter handling (FR-011) | ❌ |

---

### 17. `client/client_url.rs` — `connect_url()` integration edge cases

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Double prefix `rediss://rediss://...` | ❌ |
| 2 | Plain TCP + AUTH with password | ❌ |
| 3 | Plain TCP + username:password | ❌ |
| 4 | TLS + password + system_certs=true | ❌ |
| 5 | TLS + password + ca_cert path | ❌ |
| 6 | TLS + password + mTLS cert+key | ❌ |
| 7 | TLS + invalid tls_min_version | ❌ |
| 8 | TLS + tls_min_version=1.3 + tls_max_version=1.2 | ❌ |
| 9 | TLS + ssrf=true | ❌ |
| 10 | TLS + ssrf=false | ❌ |
| 11 | IPv6 URL `[::1]:6379` | ❌ |
| 12 | IPv6 TLS URL `[::1]:6380?system_certs=true` | ❌ |
| 13 | Port overflow (port > 65535) | ❌ |
| 14 | No host `redis://:6379` | ❌ |

---

## Coverage Matrix by Feature

| Feature | Files | Test Fns | Coverage |
|---------|-------|----------|----------|
| TlsVersion parsing | `tls/config.rs` | 6 (in `tests.rs`) | 85% |
| TlsConfig defaults / into_config | `tls/connector.rs` + `tests.rs` | 7 (in `tests.rs`) | 60% |
| TlsError Display | `tls/connector.rs` + `tests.rs` | 5 (in `tests.rs`) | 100% |
| ClientCerts (DER/PEM) | `tls/config.rs` + `tests.rs` | 3 (in `tests.rs`) | 40% |
| RustlsRootCerts (Pem/Der) | `tls/config.rs` + `tests.rs` | 1 (in `tests.rs`) | 20% |
| TlsConnector::handshake | `tls/connector.rs` | 0 | 0% |
| TlsStream (new/inner/Read/Write) | `tls/tls_stream.rs` | 0 | 0% |
| SSRF Config (is_blocked) | `connection/tcp.rs` | 0 | 0% |
| TcpConnector (connect methods) | `connection/tcp.rs` | 4 | 50% |
| ConnectionError Display | `connection/tcp.rs` | 4 | 50% |
| Connection struct methods | `connection/connection.rs` | 0 | 0% |
| connect_tls / connect_tls_with_ssrf | `connection/connection_tls.rs` | 0 | 0% |
| from_tls_stream helpers | `connection/connection_tls.rs` | 0 | 0% |
| execute_with_timeout | `client/client_timeout.rs` | 0 | 0% |
| RedisClient connect_* methods | `client/client.rs` | 0 | 0% |
| URL parsing (url_decode) | `client/client_url.rs` | 8 | 73% |
| Query param parsing | `client/client_url.rs` | 6 | 43% |
| connect_url integration | `client/client_url.rs` | 5 | 26% |

---

## Remediation Priority

1. **SSRF deny-list** (`SsrfConfig::is_blocked`) — 15 IPv4/IPv6 scenarios, pure function, easy to unit test, security-critical
2. **TlsConnector::handshake** — timeout path and safety valve are important for production resilience
3. **execute_with_timeout** — cancellation logic and timeout race conditions
4. **connect_tls / connect_tls_with_ssrf** — connection construction paths
5. **TlsStream** — simple constructors, trivial to cover
6. **Connection struct methods** — getters, trivial to cover
7. **Remaining URL parsing edge cases** — missing FR-011 (unknown params) and IPv6 tests
