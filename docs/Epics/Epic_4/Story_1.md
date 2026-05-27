# Story 4.1 — TcpConnector

**Objective:** Implement TCP connection establishment using may-aware sockets.

**Epic:** 4 — Connection Crate

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec) + Epic 3 (protocol)

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/06-connection-layer-design.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** TcpConnector resolves a host and port to one or more SocketAddr values
- [x] **FR-2:** TcpConnector creates a may-aware TcpStream from the resolved address
- [x] **FR-3:** TcpConnector sets TCP_NODELAY on the socket for low-latency response
- [x] **FR-4:** TcpConnector parses `redis://host:port` URLs for convenience
- [x] **FR-5:** All connection errors are categorized (Resolve, Connect, SetNodelay, etc.)

### Non-Functional Requirements

- [x] **NFR-1:** Connection is established within the may coroutine context
- [x] **NFR-2:** Error types implement `Display` and `std::error::Error`
- [x] **NFR-3:** The connect function iterates over DNS results

## Code Anchors

- `src/connection/tcp.rs` — `TcpConnector` and `ConnectionError`

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

- [x] Define `ConnectionError` enum with Resolve, Connect, SetNonBlock, SetNodelay, SetKeepalive variants
- [x] Implement `Display` and `Error` for `ConnectionError`
- [x] Implement `TcpConnector::connect(host, port)` — resolves address, creates socket, sets non-blocking, sets TCP_NODELAY, returns may-aware `TcpStream`
- [x] Use `socket2` for socket configuration (non-blocking, keepalive)
- [x] Add `connect_url(url: &str)` convenience that parses `redis://host:port`

## Verification

- Integration tests pass with live Redis:
  - Connection established successfully
  - TCP_NODELAY set correctly
  - URL parsing works for `redis://127.0.0.1:6379`
- `cargo clippy` — zero warnings
