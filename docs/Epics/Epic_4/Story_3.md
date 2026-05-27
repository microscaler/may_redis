# Story 4.3 — epoll connection loop body

**Objective:** Implement the actual connection loop coroutine that handles epoll events, non-blocking I/O, and response dispatch.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.2 — Connection struct with request queue must be implemented and passing tests.

**Source docs:** `docs/06-connection-layer-design.md`, `docs/02-may_postgres_comparison.md`, `docs/10-test-strategy.md`

## Requirements

### Functional Requirements

- **FR-1:** `nonblock_read(stream, read_buf)` must read from a non-blocking socket into a `BytesMut`, returning `Ok(bool)` indicating if more data is available
- **FR-2:** `nonblock_write(stream, write_buf)` must write from a `BytesMut` to a non-blocking socket, handling `WouldBlock` by breaking the loop
- **FR-3:** The connection loop coroutine must continuously: process requests → write → read → decode → epoll_wait
- **FR-4:** `process_requests(queue, write_buf, resp_queue)` must pop requests from the mpsc queue and add them to the write buffer
- **FR-5:** RESP responses must be decoded from the read buffer using `RESPReader` and dispatched to pending requests
- **FR-6:** Errors during read, write, or decode must dispatch error responses to all pending requests
- **FR-7:** Connection closed by server (read returns 0 bytes) must dispatch error responses and terminate the loop

### Non-Functional Requirements

- **NFR-1:** The connection loop runs as a single `go!` coroutine — no async/await, no tokio
- **NFR-2:** I/O operations must be non-blocking with cooperative yielding via may primitives
- **NFR-3:** `WaitIoWaker` must be used to wake the epoll loop when new requests arrive
- **NFR-4:** Buffer management must reserve space before reads to avoid allocation churn

## Code Anchors

- `crates/connection/src/connection.rs` — connection loop, `nonblock_read`, `nonblock_write`, `process_req`, RESP decoding
- `crates/connection/src/tcp.rs` — `TcpConnector::connect()` for stream creation

## Implementation Details

### nonblock_read

```rust
fn nonblock_read<R: Read>(stream: &mut R, read_buf: &mut BytesMut) -> io::Result<bool>
```

- Reserves space in read_buf (at least 512 bytes)
- Reads into buffer using the `Read` trait on may-aware streams
- Returns `true` if more data can still be read (buffer not full), `false` if read was blocked
- Handles `WouldBlock` by returning `Ok(true)` (more data available indicator)

### nonblock_write

```rust
fn nonblock_write<W: Write>(stream: &mut W, write_buf: &mut BytesMut) -> io::Result<usize>
```

- Writes from `write_buf` using the `Write` trait
- Handles `WouldBlock` by breaking the write loop, preserving remaining data in buffer
- Returns number of bytes written
- Handles zero-write as `BrokenPipe` error

### Connection Loop Algorithm

```
loop:
    1. process_req()     — pop all queued requests, add to write_buf
    2. nonblock_write()  — flush write_buf to socket
    3. nonblock_read()   — if io_events allows, read from socket
    4. decode_responses() — parse RESP from read_buf, dispatch to pending
    5. wait_io()         — epoll_wait for READABLE/WRITABLE events
```

## Acceptance Criteria

### Functional Acceptance Criteria

- [ ] **FR-1:** `nonblock_read()` successfully reads data into a `BytesMut` and returns `Ok(true)` when buffer has space
- [ ] **FR-2:** `nonblock_write()` writes data from a `BytesMut` to a socket and returns the count of bytes written
- [ ] **FR-3:** `nonblock_write()` handles `WouldBlock` by breaking the loop without losing data
- [ ] **FR-4:** `process_req()` pops all requests from the mpsc queue and appends their data to write_buf
- [ ] **FR-5:** RESPReader decodes `:1\r\n` into `RedisValue::Integer(1)`
- [ ] **FR-5:** RESPReader decodes `*2\r\n$3\r\na\r\n$3\r\nb\r\n` into `RedisValue::Array([BulkString(b"a"), BulkString(b"b")])`
- [ ] **FR-6:** `nonblock_read()` returns `Ok(false)` when socket is closed (read returns 0 bytes) → loop terminates
- [ ] **FR-7:** Connection loop spawns as a `go!` coroutine and runs indefinitely until an error

### Code Quality Acceptance Criteria

- [ ] **CQ-1:** `cargo build -p connection` — compiles without errors
- [ ] **CQ-2:** `cargo clippy -p connection --all-targets --all-features` — zero warnings
- [ ] **CQ-3:** `cargo fmt -p connection` — file formatted with no changes needed
- [ ] **CQ-4:** No `tokio` imports, no `.await`, no `async fn` anywhere in connection crate
- [ ] **CQ-5:** All public items have doc comments
- [ ] **CQ-6:** Connection loop uses only may primitives: `go!`, `may::io::WaitIo`, `may::sync::spsc`, `may::queue::mpsc::Queue`

## Verification

- `cargo test -p connection` — unit tests pass (RESP reader decode tests)
- `cargo clippy -p connection` — zero warnings
- `cargo doc -p connection --no-deps` — documentation builds without warnings
