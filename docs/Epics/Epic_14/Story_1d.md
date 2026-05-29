# Story 14.1d — connect_tls wiring

**Objective:** Wire `RedisClient::connect_tls()` end-to-end: TCP → TLS handshake → Connection spawn. Also wire `rediss://` scheme to call `connect_tls()`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 4/4 of Story 1)

**Dependencies:** Story 14.1c (TlsStream compiles)

## Deliverable

1. `Connection::from_tls_stream(tls_stream, host, timeout, ssrf_config)` — same as connect() but takes TlsStream
2. `RedisClient::connect_tls(host, port, config, timeout)` — chains TCP → TLS → Connection
3. Enable `rediss://` scheme in `connect_url()` — parse host:port, call connect_tls with system_certs=true
4. `ConnectionError::Tls(String)` variant

## Tasks

1. Patch `src/connection/tcp.rs` — add `#[cfg(feature = "tls")] Tls(String)` to ConnectionError + Display impl
2. Patch `src/connection/connection.rs` — add `Connection::from_tls_stream()` that takes TlsStream:
   - Uses `tls_stream.inner_mut()` for stream operations
   - Uses `tls_stream.inner().as_raw_fd()` for id
   - Uses `tls_stream.inner().waker()` for waker
   - Passes `tls_stream` to `spawn_connection_loop()`
3. Patch `src/client/client.rs` — add `connect_tls()` method:
   - `TcpConnector::connect_with_timeout(host, port, timeout)` → TcpStream
   - `TlsConnector::handshake(stream, config, timeout)` → TlsStream
   - `Connection::from_tls_stream(tls_stream, ...)` → Connection
   - Wrap in Arc<InnerClient> and return RedisClient
4. Patch `src/client/client.rs` — modify `connect_url()` rediss:// branch:
   - Replace "TLS is not yet supported" with actual logic
   - Parse host:port from URL (same as redis://)
   - Build TlsConfig with system_certs=true
   - Call `connect_tls()` instead of `connect()`
5. Patch `src/lib.rs` — add `#[cfg(feature = "tls")] pub use tls::{TlsConfig, TlsError, TlsVersion, ClientCerts, RustlsRootCerts};`
6. Run `cargo build --features tls`
7. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
