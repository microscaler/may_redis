# PRD — TLS and mTLS Support for may-redis

> **Status:** DRAFT — for review
> **Author:** Agent
> **Date:** 2026-07-10

---

## 1. Problem

may-redis currently connects to Redis over plain TCP. In production environments (cloud managed Redis, production clusters, PCI/HIPAA compliance) the Redis transport **must** be encrypted. The library currently returns `"TLS is not yet supported (rediss://)"` when a `rediss://` URL is encountered. There is no path to encrypted transport.

## 2. Goals

| # | Goal |
|---|------|
| G1 | Support TLS (STARTTLS/TLS-over-TCP) for Redis connections |
| G2 | Support mutual TLS (mTLS) — client certificate authentication |
| G3 | Preserve the existing connection loop architecture (epoll + `may::net::TcpStream`) — no rewriting the loop |
| G4 | Feature-flag TLS behind a Cargo feature so the default crate remains zero-dependency beyond `bytes`/`itoa`/`log`/`may`/`socket2` |
| G5 | Maintain 100% clippy deny-level compliance |
| G6 | No blocking I/O — all TLS I/O must cooperate with may's coroutine scheduler via `co_yield`/`yield_now()` |

## 3. Non-Goals (Out of Scope)

| # | Non-Goal | Rationale |
|---|----------|-----------|
| NG1 | STARTTLS (upgrade plain TCP to TLS mid-connection) | Redis does not support STARTTLS. TLS must be established during connect. |
| NG2 | TLS 1.0 / 1.1 support | RFC 8996 deprecated these. Default to 1.2+. |
| NG3 | OCSP stapling / CRL checking | Redis doesn't use OCSP; CRL is rare. Add if demand appears. |
| NG4 | Certificate pinning | Not a Redis-use pattern. |
| NG5 | TLS session resumption / ticket caching | Adds complexity. Handshake latency is dominated by network round-trips, not TLS math. |
| NG6 | Native-TLS / OpenSSL backend | `rustls` is pure Rust, no system deps, fits the may coroutine model better. |

## 4. Architecture

### 4.1 High-Level Flow

```
User code
    │
    ▼
RedisClient::connect_tls(host, port, config)
    │
    ├─→ TcpConnector::connect(host, port) → may::net::TcpStream (raw TCP)
    │
    ├─→ TlsConnector::wrap(stream, config) → TlsStream
    │        (performs TLS handshake, non-blocking)
    │
    ├─→ Connection::from_tls_stream(tls_stream) → Connection (epoll loop)
    │
    └─→ Normal command execution (unchanged)
```

### 4.2 Component Boundary Design

**The critical design decision:** Where does TLS live in the stack?

**Option A: Wrap the raw `TcpStream` — TLS before the connection loop**

```
TcpConnector::connect() → may::net::TcpStream
    │
    ▼
TlsConnector::handshake(stream, config) → TlsStream
    │
    ▼
Connection::from_stream(stream) → Connection (epoll loop reads/writes TlsStream)
```

The connection loop's `nonblock_read`/`nonblock_write` operate on the `TlsStream` instead of `TcpStream`. The epoll loop monitors the **TLS stream's underlying socket fd** (same fd as the original `TcpStream`), but reads/writes go through the TLS layer.

**Option B: TLS inside the connection loop** — no, too invasive. Would require rewriting `spawn_connection_loop` with TLS state machine integration.

**Decision: Option A.** It's the minimal change path. The connection loop remains identical — it always does `nonblock_read` + `nonblock_write` on a `Stream` trait impl. We just change what that trait impl wraps.

### 4.3 The `may::net::TcpStream` Problem

`may::net::TcpStream` wraps a raw socket and adds `wait_io()`/`waker()` for epoll integration. The `rustls` crate works with `std::net::TcpStream` via its `Stream` + `Read`/`Write` traits.

**The challenge:** `may::net::TcpStream` does NOT implement `std::io::Read`/`std::io::Write` in a way that works with rustls. Rustls expects a blocking or non-blocking `Read`/`Write` where the underlying socket is non-blocking and the caller handles `WouldBlock`.

**Solution: Implement a `rustls::Stream` adapter for `may::net::TcpStream`.**

This adapter delegates `read()`/`write()` calls to the underlying TCP stream's `Read`/`Write` (using the same `nonblock_read`/`nonblock_write` pattern the connection loop already uses). When `WouldBlock` is returned, the coroutine yields via `may::coroutine::yield_now()`, and rustls retries.

### 4.4 TLS Handshake Coroutine

The TLS handshake is the most complex part. It involves multiple alternating read/write cycles:

```
Client → Server: ClientHello
Server → Client: ServerHello + Certificate + ServerKeyExchange + ServerHelloDone
Client → Server: Certificate + ClientKeyExchange + CertificateVerify + Finished
Server → Client: Finished
```

Each message may span multiple records, and each record may require multiple socket reads. During the handshake, the socket may return `WouldBlock` at any point.

**Handshake flow in a may coroutine:**

```rust
// In the connection layer, after TCP connect succeeds:
may::go!(move || {
    let tls_conn = rustls::ClientConfig::builder()
        .with_root_certificates(root_certs)
        .with_client_auth_cert(client_cert, private_key)
        .map_err(|e| TlsError::Config(e))?;
    
    let mut stream = TlsConnector::new(tls_conn);
    
    // Perform the handshake — this yields cooperatively at each WouldBlock
    stream.connect_async(host, tcp_stream)
        .await  // NO — we cannot use async-await!
        // Must use may::co_yield / yield_now instead
});
```

**Since we cannot use async-await, the handshake must use a polling pattern:**

```rust
/// Poll-based TLS handshake using may coroutine yields.
fn handshake_poll(
    stream: &mut TlsStream,
    tcp_stream: &mut may::net::TcpStream,
    waker: &WaitIoWaker,
    timeout: Duration,
) -> Result<(), TlsError> {
    let deadline = Instant::now() + timeout;
    loop {
        match stream.handshake(tcp_stream) {
            Ok(done) => return Ok(()),
            Err(rustls::Error::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(TlsError::HandshakeTimeout);
                }
                // Yield to may scheduler so epoll can report socket readiness
                may::coroutine::yield_now();
            }
            Err(e) => return Err(TlsError::Handshake(e)),
        }
    }
}
```

This is equivalent to rustls's `async` handshake but uses `yield_now()` instead of `.await`.

## 5. API Surface

### 5.1 New Types

```rust
// Cargo feature: tls
#[cfg(feature = "tls")]
pub mod tls {
    pub struct TlsConfig {
        /// Root CA certificates (PEM or DER).
        /// If empty, use system root store (requires native-tls system deps).
        /// Default: empty — user must provide trusted CAs.
        pub root_certificates: RustlsRootCerts,
        
        /// Client certificate and key for mTLS.
        /// If None, regular TLS (server auth only).
        /// If Some, mutual TLS (client auth).
        pub client_certs: Option<ClientCerts>,
        
        /// Server hostname for SNI and certificate verification.
        /// Must match the certificate CN/SAN.
        pub server_name: String,
        
        /// Minimum TLS version (default: TLS 1.2).
        pub min_version: TlsVersion,
        
        /// Maximum TLS version (default: TLS 1.3).
        pub max_version: TlsVersion,
        
        /// Whether to verify the server certificate chain.
        /// Default: true. Set to false only for debugging.
        pub verify_server: bool,
    }
    
    pub enum RustlsRootCerts {
        /// Load from PEM-formatted certificate file(s).
        Pem(Vec<PathBuf>),
        /// Load from in-memory DER-encoded certificates.
        Der(Vec<Vec<u8>>),
        /// Use the rustls-native-certs system store.
        System,
    }
    
    pub struct ClientCerts {
        /// PEM-encoded client certificate chain (leaf first).
        pub certificates: Vec<Vec<u8>>,
        /// PEM-encoded private key.
        pub private_key: Vec<u8>,
        /// Key format (default: PEM).
        pub key_format: KeyFormat,
    }
    
    pub enum TlsVersion {
        Tls12,
        Tls13,
    }
    
    pub enum KeyFormat {
        Pem,
        Der,
    }
}
```

### 5.2 New Client Methods

```rust
impl RedisClient {
    /// Connect to a Redis server with TLS.
    ///
    /// # Arguments
    /// * `host` — Server hostname or IP address
    /// * `port` — Server port (typically 6380 for TLS)
    /// * `tls_config` — TLS configuration (root CAs, client certs, SNI)
    /// * `timeout` — Connection timeout
    ///
    /// # Example
    /// ```no_run
    /// use may_redis::{RedisClient, tls::TlsConfig, tls::RustlsRootCerts};
    ///
    /// let tls_config = TlsConfig {
    ///     root_certificates: RustlsRootCerts::Pem(vec!["/path/to/ca.pem".into()]),
    ///     client_certs: None,  // regular TLS, not mTLS
    ///     server_name: "redis.example.com".into(),
    ///     verify_server: true,
    ///     ..Default::default()
    /// };
    ///
    /// let client = RedisClient::connect_tls("redis.example.com", 6380, &tls_config, 5)?;
    /// ```
    #[cfg(feature = "tls")]
    pub fn connect_tls(
        host: &str,
        port: u16,
        tls_config: &TlsConfig,
        timeout: Duration,
    ) -> Result<Self, RedisError>;
    
    /// Connect with a URL and TLS scheme.
    ///
    /// Parses `rediss://host:port` URLs and establishes a TLS connection.
    ///
    /// # URL format
    /// * `rediss://host:port` — TLS with default port 6380
    /// * `rediss://host:port?ca=ca.pem&client_cert=cert.pem&client_key=key.pem`
    ///   — TLS with file paths (URL-encoded) for certificate material
    /// * `rediss://:password@host:port` — TLS + AUTH (password URL-decoded)
    /// * `rediss://user:password@host:port` — TLS + AUTH with username
    ///
    /// # Query parameters
    /// | Param | Required | Description |
    /// |-------|----------|-------------|
    /// | `ca` | Yes (unless `system` provided) | Path to root CA PEM file |
    /// | `client_cert` | No (for mTLS) | Path to client certificate PEM |
    /// | `client_key` | No (for mTLS) | Path to client private key PEM |
    /// | `server_name` | No | Override SNI server name (default: host from URL) |
    /// | `system_certs` | No | If `true`, use system cert store instead of `ca` param |
    /// | `verify` | No | If `false`, skip server cert verification (insecure) |
    /// | `tls_min_version` | No | `1.2` or `1.3` (default: `1.2`) |
    /// | `tls_max_version` | No | `1.2` or `1.3` (default: `1.3`) |
    ///
    /// # Example
    /// ```no_run
    /// use may_redis::RedisClient;
    ///
    /// // Simple TLS
    /// let client = RedisClient::connect_url("rediss://redis.example.com:6380")?;
    ///
    /// // TLS with mTLS and custom SNI
    /// let client = RedisClient::connect_url(
    ///     "rediss://redis.example.com:6380?ca=/path/to/ca.pem&client_cert=/path/to/client.pem&client_key=/path/to/client.key&server_name=redis.internal"
    /// )?;
    /// ```
    #[cfg(feature = "tls")]
    pub fn connect_url(url: &str) -> Result<Self, RedisError>;
    // ^ Already exists — just needs the TLS branch enabled
}
```

### 5.3 Connection Layer Changes

```rust
// connection/connection.rs additions:
#[cfg(feature = "tls")]
impl Connection {
    /// Create a Connection from a TLS-wrapped stream.
    ///
    /// The TLS handshake MUST already be complete before calling this.
    /// The epoll loop will wrap the TLS stream the same way it wraps TCP.
    #[allow(dead_code)] // Used when TLS feature is enabled
    fn from_tls_stream(
        stream: TlsStream,
        host: &str,
        timeout: Duration,
        max_queue_depth: usize,
        max_request_size: usize,
        ssrf_config: Option<tcp::SsrfConfig>,
    ) -> Result<Self, ConnectionError>;
}

// New error variant:
pub enum ConnectionError {
    // ... existing variants ...
    #[cfg(feature = "tls")]
    Tls(String),
}
```

## 6. Dependency Changes

### 6.1 Cargo.toml

```toml
[features]
default = []
tls = ["dep:rustls", "dep:rustls-pemfile", "dep:rustls-native-certs", "dep:webpki-roots"]
test = []

[dependencies]
# ... existing ...
rustls = { version = "0.23", optional = true, default-features = false, features = ["ring"] }
rustls-pemfile = { version = "2", optional = true }
rustls-native-certs = { version = "0.8", optional = true }
webpki-roots = { version = "0.26", optional = true }
```

### 6.2 Why `rustls` + `ring`?

| Criteria | `rustls` + `ring` | `rustls` + `aws-lc-rs` | `native-tls` | `openssl` |
|----------|--------------------|------------------------|-------------|-----------|
| Pure Rust | Yes | Yes | No (system OpenSSL) | No (system OpenSSL) |
| Native deps | None | None | Requires OpenSSL headers | Requires OpenSSL |
| macOS build | Works | Works | Needs brew | Needs brew |
| Linux build | Works | Works | Needs libssl-dev | Needs libssl-dev |
| Cryptographic quality | Production-proven | Production-proven (AWS) | Mature | Mature |
| WASM support | Yes | Yes | No | No |
| Ring binary size | ~1.2 MB (release) | ~0.8 MB | N/A | N/A |
| MSRV | Rust 1.63+ | Rust 1.63+ | N/A | N/A |

**Decision:** `ring` is the default crypto provider. It's the most battle-tested for rustls and has no system dependencies, which is critical for a library that needs to build everywhere.

If bandwidth is a concern, `aws-lc-rs` can be offered as an alternative via a Cargo feature: `tls-ring` (default) and `tls-aws-lc`.

## 7. Implementation Plan

### Story T1: TLS Foundation — TlsConnector and Handshake

**Objective:** Implement the TLS handshake using a polling pattern with may coroutine yields.

**Tasks:**
1. Add `tls` Cargo feature with `rustls`, `rustls-pemfile`, `webpki-roots` dependencies
2. Create `src/tls/mod.rs` — `TlsConfig`, `TlsError`, `RustlsRootCerts` types
3. Implement `TlsConnector::handshake()` — polling-based TLS handshake that yields cooperatively
4. Implement `TlsStream` — wraps `rustls::Stream` and implements `Read`/`Write` for non-blocking I/O
5. Wire `RedisClient::connect_tls()` — connects TCP → handshakes TLS → creates Connection

**Verification:** Unit tests with a mock TLS server (or `openssl s_server`), integration test against Redis with `--tls-port`

### Story T2: mTLS Support

**Objective:** Add client certificate authentication.

**Tasks:**
1. Extend `TlsConfig` with `client_certs: Option<ClientCerts>`
2. Implement PEM/DER loading for client certificates and private keys
3. Wire client certs into `rustls::ClientConfig::with_client_auth_cert()`
4. Add URL query parameter support for `client_cert` and `client_key`

**Verification:** Integration test against Redis configured with `tls-auth-clients yes` and `tls-client-cert-auth yes`

### Story T3: URL Parsing for rediss://

**Objective:** Enable `rediss://` URLs with certificate path query parameters.

**Tasks:**
1. Modify `connect_url()` in `client.rs` to handle `rediss://` scheme (currently rejected)
2. Parse query parameters: `ca`, `client_cert`, `client_key`, `server_name`, `system_certs`, `verify`, `tls_min_version`, `tls_max_version`
3. Convert query parameters to `TlsConfig`
4. Add comprehensive tests for URL parsing

**Verification:** Unit tests for all URL combinations, including edge cases (missing params, invalid paths, special characters)

### Story T4: SSRF Protection for TLS Connections

**Objective:** SSRF checks must still apply when TLS is enabled.

**Tasks:**
1. Ensure `Connection::connect_tls()` accepts and applies `SsrfConfig`
2. SSRF check runs on the resolved IP before TLS handshake (same as plain TCP)
3. Document that SSRF checks apply regardless of TLS

**Verification:** Integration test: TLS connection to `rediss://127.0.0.1:6379` with SSRF protection should fail

### Story T5: TLS Configuration Options

**Objective:** Provide configurable TLS versions and cipher suites.

**Tasks:**
1. Add `min_version`/`max_version` to `TlsConfig` (default: 1.2–1.3)
2. Add `cipher_suites` override (optional — let rustls use defaults unless user overrides)
3. Add `alpn_protocols` support (for future Redis+HTTP2 if needed; Redis over TLS uses plain RESP)
4. Add connection timeout for TLS handshake (separate from command timeout)

**Verification:** Test that `min_version = TLS1.3` correctly rejects a TLS 1.2-only server

### Story T6: Certificate Reload (Optional / V2)

**Objective:** Hot-reload certificates without reconnecting.

**Tasks:**
- **Postponed to V2** — requires significant restructuring of `TlsStream` to use `Arc`-shared config that can be swapped.

## 8. Connection Loop Impact Analysis

### 8.1 What changes in the loop

The connection loop in `spawn_connection_loop` does:

```rust
let inner = stream.inner_mut();
// ...
nonblock_write(inner, &mut write_buf)
nonblock_read(inner, &mut read_buf)
```

With TLS, `inner_mut()` returns `&mut TlsStream`. The `TlsStream` wraps both the rustls state machine AND the underlying `may::net::TcpStream`.

**`nonblock_write` with TLS:**
- Calls `tls_stream.write()` → rustls may:
  - Write buffered plaintext (immediate)
  - Need to flush TLS record to socket (calls through to TCP)
  - Return `WouldBlock` if TCP buffer full or TLS buffer not yet ready
- `nonblock_write` handles `WouldBlock` correctly — it already breaks on `WouldBlock`

**`nonblock_read` with TLS:**
- Calls `tls_stream.read()` → rustls may:
  - Decrypt and return buffered data (immediate)
  - Need to read TLS record from socket (calls through to TCP)
  - Return `WouldBlock` if no data available yet
- `nonblock_read` handles `WouldBlock` correctly — already the documented behavior

**Conclusion:** The connection loop does NOT need changes. `TlsStream` just needs to implement `Read` and `Write` such that `nonblock_read` and `nonblock_write` work as-is. The loop already handles `WouldBlock` correctly — it's the expected path for non-blocking I/O.

### 8.2 The `stream.wait_io()` call

`stream.wait_io()` is called on `may::net::TcpStream`. With TLS, we need to call `wait_io()` on the **underlying TCP socket**, not on the TLS wrapper.

**Solution:** `TlsStream` exposes `.raw_fd()` or `.inner()` that returns a reference to the underlying `may::net::TcpStream`. The connection loop calls:

```rust
// With TLS:
let tls_stream: TlsStream;
let raw = tls_stream.inner_mut(); // returns &mut may::net::TcpStream
raw.wait_io()  // polls epoll on the TCP socket fd
```

The TLS fd is the same as the TCP fd — we're not changing the file descriptor, just adding a TLS layer on top.

## 9. Test Strategy

### 9.1 Unit Tests (no network)

| Test | Purpose |
|------|---------|
| `test_tls_config_defaults` | Verify default TlsConfig values |
| `test_tls_config_min_version_13` | TLS 1.3-only config creates correct rustls config |
| `test_tls_config_invalid_version` | Reject TLS 1.1 config |
| `test_url_parse_rediss_basic` | `rediss://host:6380` parses correctly |
| `test_url_parse_rediss_mtls` | mTLS URL with query params parses correctly |
| `test_url_parse_rediss_system_certs` | `system_certs=true` uses system store |
| `test_url_parse_rediss_query_encoding` | Special chars in paths are URL-decoded |
| `test_url_parse_rediss_no_ca_fails` | No CA + no system_certs returns error |
| `test_tls_handshake_timeout` | Handshake times out correctly |
| `test_tls_handshake_verify_fail` | Self-signed cert fails verification |
| `test_tls_handshake_verify_ok` | Trusted CA cert succeeds |

### 9.2 Integration Tests (live Redis with TLS)

Require a Redis instance with TLS configured:

```bash
# Generate test certificates
openssl req -x509 -newkey rsa:2048 -keyout ca.key -out ca.crt -days 365 -nodes -subj "/CN=TestCA"
openssl req -newkey rsa:2048 -keyout server.key -out server.csr -nodes -subj "/CN=localhost"
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365
# Redis config: tls-port 6381, tls-cert-file server.crt, tls-key-file server.key, tls-ca-file ca.crt
```

| Test | Purpose |
|------|---------|
| `test_tls_connect_and_ping` | Connect over TLS and send PING |
| `test_tls_mtls_connect` | Connect with client cert and authenticate |
| `test_tls_set_get` | Full round-trip SET/GET over TLS |
| `test_tls_pipeline` | Pipeline of 10 commands over TLS |
| `test_tls_timeout` | Command timeout works over TLS |
| `test_tls_server_cert_rejection` | Connecting with wrong CA fails |
| `test_tls_self_signed_rejection` | Connecting without trusted CA fails |
| `test_tls_cert_expired` | Expired server cert is rejected |
| `test_tls_wrong_hostname` | SNI mismatch is rejected |

## 10. Performance Considerations

### 10.1 Handshake Latency

TLS 1.2 full handshake: 1-2 RTTs. TLS 1.3: 1 RTT.

For `may-redis` this is a one-time cost per connection. The connection pool (if added later) would amortize this. For now, each `connect_tls()` pays the full handshake cost.

**Acceptable:** ~50-200ms for handshake on a well-connected network.

### 10.2 Per-Message Overhead

TLS adds ~5-30 bytes per record (header + AEAD tag). For small RESP messages (single SET/GET), overhead is ~10-20%. For pipeline batches, overhead is amortized.

**Not a concern for v1.** Redis messages are typically 100+ bytes, so TLS overhead is <10%.

### 10.3 Memory Impact

`rustls` with `ring` adds ~1.2 MB binary size. Runtime memory per TLS connection: ~64-128 KB for TLS session state + read/write buffers.

**Acceptable.** No TLS feature = zero impact.

### 10.4 Throughput

TLS adds CPU for encryption/decryption. On modern CPUs with AES-NI, throughput is >1 Gbps per core. For Redis workloads (typically sub-Gbps), TLS encryption is **not the bottleneck**.

## 11. Security Considerations

### 11.1 Certificate Verification

**MUST VERIFY** server certificates by default. The `verify_server: false` option is a debugging only feature, clearly documented as insecure.

### 11.2 TLS Version Policy

- **Minimum: TLS 1.2** — TLS 1.0/1.1 are deprecated (RFC 8996)
- **Maximum: TLS 1.3** — current standard
- rustls defaults to TLS 1.2 minimum; users can raise to 1.3 only

### 11.3 Cipher Suite Selection

rustls has a built-in cipher suite list that only includes secure ciphers (AES-GCM, ChaCha20-Poly1305). No user-configurable cipher suites needed in v1 — rustls defaults are production-secure.

### 11.4 mTLS Certificate Handling

- Private keys must be loadable from PEM files — never hardcoded
- File paths in URLs must be URL-encoded (already handled by `connect_url`)
- Key permissions are the caller's responsibility

### 11.5 Handshake Timeout

TLS handshake must have its own timeout (default: 5 seconds, same as TCP connect). A hung handshake is a denial-of-service vector.

## 12. API Compatibility

This change is **additive only**. No existing API is modified or removed:

- `connect()` — unchanged, plain TCP
- `connect_with_timeout()` — unchanged, plain TCP
- `connect_with_ssrf_protection()` — unchanged, plain TCP
- `connect_url()` — already exists; `redis://` still works; `rediss://` is new

The `ConnectionScheme` enum already has `Tls` variant (currently `#[allow(dead_code)]`).

## 13. Implementation Order

```
T1: TLS Foundation → T2: mTLS → T3: URL Parsing → T4: SSRF for TLS → T5: Config Options → T6: Cert Reload (V2)
```

T1 is the dependency blocker. T2 can begin once T1's `TlsConfig` is stable. T3 can begin in parallel with T1 since it's mostly URL parsing logic.

## 14. Success Criteria

| Criterion | Measure |
|-----------|---------|
| TLS connectivity | Can connect to Redis with `tls-port` enabled and execute commands |
| mTLS connectivity | Can connect to Redis with `tls-auth-clients yes` and authenticate |
| No plain-text traffic | All traffic encrypted — verifiable with `tcpdump` |
| Feature-gated | `cargo build` without `--features tls` has zero new dependencies |
| Clippy clean | `cargo clippy --all-features` passes at deny level |
| Test coverage | All new code covered by unit + integration tests |
| SSRF still works | TLS connections are subject to the same SSRF checks |
| Docs | Public API is documented with examples |
