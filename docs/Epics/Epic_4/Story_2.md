# Story 4.2 — Connection struct with request queue

**Objective:** Implement the Connection struct that owns the request queue and spawns the connection loop.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.1 — TcpConnector must be implemented and passing tests.

**Source docs:** `docs/06-connection-layer-design.md`, `docs/02-may_postgres_comparison.md`

## Requirements

### Functional Requirements

- **FR-1:** Connection struct must own a `JoinHandle<()>` for the connection loop coroutine
- **FR-2:** Connection struct must own an `Arc<Queue<Request>>` for the request queue
- **FR-3:** Connection struct must own a `WaitIoWaker` to signal new requests
- **FR-4:** `Connection::connect(host, port)` must establish TCP, spawn the connection loop, and return Connection
- **FR-5:** `Connection::send(&self, request: Request)` must push to the request queue and wake the loop
- **FR-6:** `Connection::id(&self)` must return a unique identifier for the connection
- **FR-7:** `Drop` for Connection must cancel the connection loop coroutine

### Non-Functional Requirements

- **NFR-1:** Request queue must be safe to push from multiple coroutines (Arc + mpsc Queue)
- **NFR-2:** Tag counter must use `AtomicUsize` with SeqCst ordering for request-response matching
- **NFR-3:** Connection drop must be synchronous — the coroutine is cancelled immediately, no graceful shutdown

## Code Anchors

- `crates/connection/src/lib.rs` — `pub struct Connection`, `pub struct Request`
- `crates/connection/src/connection.rs` — implementation

## Structs

```rust
pub struct Request {
    pub data: Vec<u8>,
    pub sender: spsc::Sender<RedisValue>,
}

pub struct Connection {
    io_handle: JoinHandle<()>,
    req_queue: Arc<Queue<Request>>,
    waker: WaitIoWaker,
    id: usize,
    tag_counter: Arc<AtomicUsize>,
}

impl Connection {
    pub fn connect(host: &str, port: u16) -> Result<Self, ConnectionError>;
    pub fn send(&self, request: Request) -> usize;
    pub fn id(&self) -> usize;
}
```

## Tasks

1. Define `Request` struct with data and sender fields
2. Define `Connection` struct with io_handle, req_queue, waker, id, tag_counter fields
3. Implement `Connection::connect(host, port)` — establishes TCP via TcpConnector, spawns epoll loop, returns Connection
4. Implement `Connection::send(&self, request: Request)` — pushes to req_queue, signals waker, returns tag
5. Implement `Connection::id(&self)` — returns connection id
6. Implement `Drop for Connection` — cancels the connection loop coroutine

## Acceptance Criteria

### Functional Acceptance Criteria

- [ ] **FR-1:** Connection struct has an `io_handle: JoinHandle<()>` field
- [ ] **FR-2:** Connection struct has an `req_queue: Arc<Queue<Request>>` field
- [ ] **FR-3:** Connection struct has a `waker: WaitIoWaker` field
- [ ] **FR-4:** `Connection::connect("127.0.0.1", 6379)` (with Redis running) returns `Ok(Connection)`
- [ ] **FR-5:** `Connection::send(request)` returns a monotonically increasing tag
- [ ] **FR-6:** Multiple calls to `Connection::send()` return incrementing tags (0, 1, 2, ...)
- [ ] **FR-7:** Connection's `Drop` implementation calls `coroutine().cancel()` on the io_handle

### Code Quality Acceptance Criteria

- [ ] **CQ-1:** `cargo build -p connection` — compiles without errors
- [ ] **CQ-2:** `cargo clippy -p connection --all-targets --all-features` — zero warnings
- [ ] **CQ-3:** `cargo fmt -p connection` — file formatted with no changes needed
- [ ] **CQ-4:** All public items have doc comments
- [ ] **CQ-5:** No `unwrap()` or `expect()` in production code (clippy `unwrap_used` is deny)

## Verification

- `cargo test -p connection` — unit tests pass
- `cargo clippy -p connection` — zero warnings
- `cargo doc -p connection --no-deps` — documentation builds without warnings
