# Story 4.1 — TcpConnector

**Objective:** Implement TCP connection establishment using may-aware sockets.

**Epic:** 4 — Connection Crate

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec) + Epic 3 (protocol)

**Source docs:** `docs/06-connection-layer-design.md`

## Requirements

### Functional Requirements

- **FR-1:** TcpConnector must resolve a host and port to one or more SocketAddr values
- **FR-2:** TcpConnector must create a may-aware TcpStream from the resolved address
- **FR-3:** TcpConnector must set TCP_NODELAY on the socket for low-latency response
- **FR-4:** TcpConnector must parse `redis://host:port` URLs for convenience
- **FR-5:** All connection errors must be categorized (Resolve, Connect, SetNodelay, etc.)

### Non-Functional Requirements

- **NFR-1:** Connection must be established within the may coroutine context (cooperative yielding)
- **NFR-2:** Error types must implement `Display` and `std::error::Error`
- **NFR-3:** The connect function must retry on the first successful address from DNS resolution

## Code Anchors

- `crates/connection/src/lib.rs` — `pub struct TcpConnector`
- `crates/connection/src/tcp.rs` — implementation

## Structs

```rust
pub struct TcpConnector;

impl TcpConnector {
    pub fn connect(host: &str, port: u16) -> Result<TcpStream, ConnectionError>;
    pub fn connect_url(url: &str) -> Result<TcpStream, ConnectionError>;
}

pub enum ConnectionError {
    Resolve(String),
    Connect(String),
    SetNonBlock(String),
    SetNodelay(String),
    SetKeepalive(String),
}
```

## Tasks

1. Define `ConnectionError` enum with Resolve, Connect, SetNonBlock, SetNodelay, SetKeepalive variants
2. Implement `Display` and `Error` for `ConnectionError`
3. Implement `TcpConnector::connect(host, port)` — resolves address, creates socket, sets non-blocking, sets TCP_NODELAY, returns may-aware `TcpStream`
4. Use `socket2` for socket configuration (non-blocking, keepalive)
5. Add `connect_url(url: &str)` convenience that parses `redis://host:port`

## Acceptance Criteria

### Functional Acceptance Criteria

- [ ] **FR-1:** TcpConnector::connect() successfully resolves `127.0.0.1` to a SocketAddr
- [ ] **FR-2:** TcpConnector::connect() returns a `may::net::TcpStream` on successful connection
- [ ] **FR-3:** TcpConnector::connect() sets TCP_NODELAY on the socket (verified via `get_nodelay()` in tests or inspection)
- [ ] **FR-4:** `TcpConnector::connect_url("redis://127.0.0.1:6379")` parses host and port correctly
- [ ] **FR-5:** ConnectionError::Resolve variants display "resolve" in their error message
- [ ] **FR-5:** ConnectionError::Connect variants display "connect" in their error message
- [ ] **FR-5:** ConnectionError::SetNodelay variants display "nodelay" in their error message

### Code Quality Acceptance Criteria

- [ ] **CQ-1:** `cargo test -p connection` passes with at least 4 unit tests:
  - `test_tcp_connector_struct_exists` — TcpConnector is constructible (no actual connect)
  - `test_connection_error_display` — all error variants format correctly
  - `test_resolve_ip_address` — resolving `127.0.0.1` returns the correct port
  - `test_connect_url_parses` — URL parsing works with unresolvable host
- [ ] **CQ-2:** `cargo clippy -p connection --all-targets --all-features` — zero warnings
- [ ] **CQ-3:** `cargo fmt -p connection` — file formatted with no changes needed
- [ ] **CQ-4:** All public items have doc comments

## Verification

- `cargo test -p connection` — at least 4 unit tests passing
- `cargo clippy -p connection` — zero warnings
- `cargo doc -p connection --no-deps` — documentation builds without warnings
