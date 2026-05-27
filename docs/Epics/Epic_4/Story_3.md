# Story 4.3 — epoll connection loop body

**Objective:** Implement the actual connection loop coroutine that handles epoll events, non-blocking I/O, and response dispatch.

**Epic:** 4 — Connection Crate

**Dependencies:** Story 4.2

**Source docs:** `docs/06-connection-layer-design.md`

## Code Anchors

- `crates/connection/src/loop.rs` — the main connection loop
- `crates/connection/src/io.rs` — non-blocking read/write helpers

## Tasks

1. Implement `nonblock_read(stream, read_buf)` — reads into BytesMut, returns bool (more data available)
2. Implement `nonblock_write(stream, write_buf)` — writes from BytesMut, handles WouldBlock
3. Implement `decode_responses(read_buf, resp_queue, codec)` — uses RESPReader to decode buffered data, dispatches to correct spsc channel via tag matching
4. Implement `process_requests(queue, write_buf, resp_queue, tag_counter)` — pops requests, adds to resp_queue and write_buf
5. Implement the main `connection_loop(stream, req_queue, waker)` — the `go!` coroutine:
   - Loop: process_requests → nonblock_write → nonblock_read → decode_responses → epoll_wait
   - Priority: READABLE → read/decode/dispatch; WRITABLE → process requests/write

## Verification

- `cargo test -p connection` — at least 3 unit tests:
  - `test_decode_simple_response` — `:1\r\n` → Integer(1) via RESPReader
  - `test_decode_array_response` — `*2\r\n$3\r\na\r\n$3\r\nb\r\n` → Array([BulkString("a"), BulkString("b")])
  - `test_nonblock_read_wouldblock` — simulates WouldBlock, returns false
- `cargo clippy -p connection` — zero warnings
