# Story 4.2 — Connection struct with request queue

**Objective:** Implement the Connection struct that owns the request queue and spawns the connection loop.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.1

**Source docs:** `docs/06-connection-layer-design.md`

## Code Anchors

- `crates/connection/src/lib.rs` — `pub struct Connection`
- `crates/connection/src/connection.rs` — implementation
- `crates/connection/src/queue.rs` — request queue management

## Structs

```rust
use may::queue::mpsc::Queue;

pub struct Connection {
    io_handle: JoinHandle<()>,
    req_queue: Arc<Queue<Request>>,
    waker: WaitIoWaker,
    id: usize,
}
```

## Tasks

1. Define `Request` struct (mirrors protocol crate's Request, but this crate owns the actual struct used in the queue)
2. Define `Connection` struct with io_handle, req_queue, waker, id fields
3. Implement `Connection::connect(host, port)` — establishes TCP connection, spawns epoll loop, returns Connection
4. Implement `Connection::send(&self, request: Request)` — pushes to req_queue, signals waker
5. Implement `Connection::id(&self)` — returns connection id
6. Implement `Drop` for Connection — gracefully closes the connection loop

## Verification

- `cargo test -p connection` — at least 3 unit tests:
  - `test_connection_struct_fields` — Connection fields are accessible
  - `test_request_struct_fields` — Request fields are accessible
  - `test_queue_creation` — mpsc::Queue<Request> is constructible
- `cargo clippy -p connection` — zero warnings
