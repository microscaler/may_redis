# Story 4.1 — TcpConnector

**Objective:** Implement TCP connection establishment using may-aware sockets.

**Epic:** 4 — Connection Crate

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec) + Epic 3 (protocol)

**Source docs:** `docs/06-connection-layer-design.md`

## Code Anchors

- `crates/connection/src/lib.rs` — `pub struct TcpConnector`
- `crates/connection/src/tcp.rs` — implementation

## Structs

```rust
pub struct TcpConnector;

impl TcpConnector {
    pub fn connect(host: &str, port: u16) -> Result<TcpStream, ConnectionError>;
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

## Verification

- `cargo test -p connection` — at least 2 unit tests:
  - `test_tcp_connector_struct_exists` — TcpConnector is constructible (no actual connect)
  - `test_connection_error_display` — error formatting
- `cargo clippy -p connection` — zero warnings
