# Story 4.2 — Connection struct with request queue

**Objective:** Implement the Connection struct that owns the request queue and spawns the connection loop.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.1 — TcpConnector

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/06-connection-layer-design.md`, `docs/02-may_postgres_comparison.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** Connection struct owns a `JoinHandle<()>` for the connection loop coroutine
- [x] **FR-2:** Connection struct owns an `Arc<Queue<Request>>` for the request queue
- [x] **FR-3:** Connection struct owns a `WaitIoWaker` to signal new requests
- [x] **FR-4:** `Connection::connect(host, port)` establishes TCP, spawns the connection loop, returns Connection
- [x] **FR-5:** `Connection::send(&self, request: Request)` pushes to the request queue and wakes the loop
- [x] **FR-6:** `Connection::id(&self)` returns a unique identifier for the connection
- [x] **FR-7:** `Drop` for Connection cancels the connection loop coroutine

### Non-Functional Requirements

- [x] **NFR-1:** Request queue is safe to push from multiple coroutines (Arc + mpsc Queue)
- [x] **NFR-2:** Tag counter uses `AtomicUsize` for request-response matching
- [x] **NFR-3:** Connection drop cancels the coroutine immediately

## Code Anchors

- `src/connection/connection.rs` — `Connection`, `Request`, `TagCounter`

## Structs

```rust
pub struct Connection {
    io_handle: JoinHandle<()>,
    req_queue: Arc<Queue<Request>>,
    waker: WaitIoWaker,
    id: usize,
    tag_counter: Arc<AtomicUsize>,
}
```

## Tasks

- [x] Define `Request` struct with data and sender fields
- [x] Define `Connection` struct with io_handle, req_queue, waker, id, tag_counter fields
- [x] Implement `Connection::connect(host, port)` — establishes TCP via TcpConnector, spawns epoll loop, returns Connection
- [x] Implement `Connection::send(&self, request: Request)` — pushes to req_queue, signals waker, returns tag
- [x] Implement `Connection::id(&self)` — returns connection id
- [x] Implement `Drop for Connection` — cancels the connection loop coroutine

## Verification

- All 11 integration tests pass (requires Redis on localhost:6379)
- Monotonically increasing tags: 0, 1, 2, ...
- Connection drop terminates coroutine cleanly
- `cargo clippy` — zero warnings
