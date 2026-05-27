# Story 4.3 — epoll connection loop body

**Objective:** Implement the actual connection loop coroutine that handles epoll events, non-blocking I/O, and response dispatch.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.2 — Connection struct with request queue

**Status:** COMPLETE — all tasks implemented and tested.

**Source docs:** `docs/06-connection-layer-design.md`, `docs/02-may_postgres_comparison.md`, `docs/10-test-strategy.md`

## Requirements

### Functional Requirements

- [x] **FR-1:** `nonblock_read(stream, read_buf)` reads from a non-blocking socket into a `BytesMut`
- [x] **FR-2:** `nonblock_write(stream, write_buf)` writes from a `BytesMut` to a non-blocking socket
- [x] **FR-3:** The connection loop continuously: process requests → write → read → decode → epoll_wait
- [x] **FR-4:** `process_requests(queue, write_buf, resp_queue)` pops requests from the mpsc queue
- [x] **FR-5:** RESP responses decoded from the read buffer using `RESPReader` and dispatched to pending requests
- [x] **FR-6:** Errors during read, write, or decode dispatch error responses to all pending requests
- [x] **FR-7:** Connection closed by server (read returns 0 bytes) dispatches error responses and terminates the loop

### Non-Functional Requirements

- [x] **NFR-1:** The connection loop runs as a single `go!` coroutine — no async/await, no tokio
- [x] **NFR-2:** I/O operations are non-blocking with cooperative yielding via may primitives
- [x] **NFR-3:** `WaitIoWaker` is used to wake the epoll loop when new requests arrive
- [x] **NFR-4:** Buffer management reserves space before reads

## Code Anchors

- `src/connection/connection.rs` — connection loop, `nonblock_read`, `nonblock_write`, `process_req`
- `src/connection/epoll.rs` — epoll event handling

## Implementation Details

```
loop:
    1. process_req()     — pop all queued requests, add to write_buf
    2. nonblock_write()  — flush write_buf to socket
    3. nonblock_read()   — if io_events allows, read from socket
    4. decode_responses() — parse RESP from read_buf, dispatch to pending
    5. wait_io()         — epoll_wait for READABLE/WRITABLE events
```

## Tasks

- [x] Implement `nonblock_read()` — reserves space, reads into BytesMut, returns bool for more-data-available
- [x] Implement `nonblock_write()` — writes from BytesMut, handles WouldBlock
- [x] Implement `process_requests()` — pops requests from mpsc queue, appends to write_buf
- [x] Implement RESP decoding via `RESPReader` — dispatches to pending Request spsc receivers
- [x] Implement error handling — errors propagate to pending requests, connection close terminates loop
- [x] Implement connection loop as `go!` coroutine with epoll event loop

## Verification

- All integration tests pass — connection loop sends and receives correctly
- No tokio imports, no `.await`, no `async fn` anywhere in the codebase
- Uses only may primitives: `go!`, `may::io::WaitIo`, `may::sync::spsc`, `may::queue::mpsc::Queue`
- `cargo clippy` — zero warnings
