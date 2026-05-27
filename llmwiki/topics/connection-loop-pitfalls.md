# Connection Loop Pitfalls

- Status: verified
- Source docs: `docs/02-may_postgres_comparison.md`, `docs/06-connection-layer-design.md`
- Code anchors:
  - `src/connection/connection.rs` — `spawn_connection_loop`, `decode_responses`, `nonblock_read`, `nonblock_write`
  - `../may_postgres/src/connection.rs` — `connection_loop`, `decode_messages` (reference)
- Last updated: 2026-05-27

This page is a running list of subtle bugs that have actually been observed
in the may-redis connection loop, with the root cause, the fix, and a
regression test for each. The connection loop is the single most fragile
piece of the codebase (single coroutine running an epoll loop, hand-rolled
non-blocking I/O, manual response demultiplexing); any change there must
be checked against this page first.

The reference implementation is **always** `../may_postgres/src/connection.rs`.
When in doubt, compare against it.

## Bug 1 — Connection loop never yields to epoll (integration tests hang)

### Symptom

- All `client::client::tests::test_integration_*` tests (PING, SET/GET,
  INCR, KEYS, DBSIZE, pipeline, concurrent, …) hang indefinitely after a
  TCP connection is established. Process must be killed with `timeout` /
  SIGTERM. Unit tests (codec, core, protocol, in_memory) all pass — the
  hang only appears for code paths that actually drive the connection loop.
- Single-test reproducer:
  `cargo test client::client::tests::test_integration_ping -- --test-threads=1`

### Root cause

In `spawn_connection_loop` the result of `nonblock_read` was discarded and
`read_blocked` was hardcoded to `false`:

```rust
let read_blocked = if io_events & 1 != 0 {
    if let Err(e) = nonblock_read(inner, &mut read_buf) {
        // error handling
        break;
    }
    false                  // BUG — should be the Ok(bool) value
} else {
    true
};

io_events = if read_blocked || !write_buf.is_empty() {
    stream.wait_io()
} else {
    1
};
```

`nonblock_read` returns `Ok(true)` when the socket returned `WouldBlock`
(no more data right now — we **must** wait on epoll) and `Ok(false)` when
the read filled the buffer completely (re-read immediately). The buggy
branch always reported "buffer filled, read more" even when the socket
had no more data, so the loop took the `else { 1 }` branch and re-ran
`nonblock_read` forever, never reaching `stream.wait_io()`.

Because the connection loop is a coroutine and `wait_io()` is its only
yield point, the loop hogged its worker thread. The test coroutine
sharing the same worker therefore never got to run
`connection.send(request)` / `rx.recv()`, and even if the request did
get pushed from a different worker, no epoll readable event was ever
awaited so the response read could not be scheduled. Result: deadlock.

### Fix

Capture and propagate the `bool` returned by `nonblock_read`:

```rust
let read_blocked = if io_events & 1 != 0 {
    match nonblock_read(inner, &mut read_buf) {
        Ok(blocked) => blocked,
        Err(e) => {
            // drain pending senders with the error then exit the loop
            break;
        }
    }
} else {
    true
};
```

### Regression coverage

- All 11 `client::client::tests::test_integration_*` tests act as
  end-to-end regressions; they wedge in seconds without the fix and
  finish in tens of milliseconds with it.
- See `src/connection/connection.rs::tests::test_connection_connect`
  and friends for the lower-level coverage.

### Lesson

`nonblock_read` / `nonblock_write` carry critical state in their return
value. Never discard them. If a future refactor changes the signature,
update both call sites in lockstep with `decode_responses`.

## Bug 2 — `decode_responses` dropped trailing bytes after a successful decode (pipeline hang)

### Symptom

- `client::client::tests::test_integration_pipeline` (and any scenario
  where several RESP responses arrive in one TCP read) hangs on
  `rx.recv()` for every response after the first one. The first
  response is delivered correctly; the rest of the `PendingRequest`
  senders are never signalled and the `spsc::Receiver`s block forever.

### Root cause

`decode_responses` constructed a fresh `RESPReader` from
`read_buf.split()` (which empties `read_buf`), decoded **one** value,
and dropped the reader without putting the unconsumed tail back. The
error path restored the bytes via `reader.take_buf()` /
`read_buf.unsplit(...)`; the success path did not:

```rust
while !read_buf.is_empty() {
    let mut reader = RESPReader::new(read_buf.split()); // read_buf is now empty
    match reader.read_value() {
        Ok(value) => {
            // BUG — reader (and its remaining bytes) is dropped here.
            // read_buf stays empty, the outer while loop exits, and
            // every subsequent batched response is lost.
            if let Some(pending) = resp_queue.pop_front() {
                let _ = pending.sender.send(value);
            }
        }
        Err(RedisError::Parse(_)) => {
            read_buf.unsplit(reader.take_buf());
            break;
        }
        Err(e) => {
            read_buf.unsplit(reader.take_buf());
            return Err(io::Error::other(e));
        }
    }
}
```

Why this only bites under load / under pipelines: a single-command
`execute()` waits for the response before sending the next command, so
the kernel almost always delivers exactly one RESP value per read. The
pipeline flushes N requests at once; the server happily returns all N
responses in one TCP segment, the loop reads them in a single
`nonblock_read`, and only the first one is dispatched.

### Fix

Put the unconsumed tail back into `read_buf` on the success path as well:

```rust
Ok(value) => {
    read_buf.unsplit(reader.take_buf());
    if let Some(pending) = resp_queue.pop_front() {
        let _ = pending.sender.send(value);
    } else {
        log::warn!("unexpected response from server");
    }
}
```

The outer `while !read_buf.is_empty()` then naturally drains the
remaining responses one by one.

### Regression coverage

- `src/connection/connection.rs::tests::test_decode_responses_multiple_in_one_buffer`
  — four concatenated responses, all four pending senders must receive
  their value and the buffer must drain to empty.
- `src/connection/connection.rs::tests::test_decode_responses_multiple_with_partial_trailing`
  — two full responses followed by a partial bulk string; the two full
  responses dispatch, the partial bytes stay in `read_buf` for the next
  read, and one `PendingRequest` remains queued.
- `client::client::tests::test_integration_pipeline` is the end-to-end
  regression for the original symptom.

### Lesson

`BytesMut::split()` is destructive — once you call it, `read_buf` is
empty until you `unsplit` something back into it. Every branch that
calls `read_buf.split()` must end by either consuming all the bytes,
or putting the leftover back via `take_buf()` + `unsplit()`. A safer
long-term refactor would teach `RESPReader` to operate on
`&mut BytesMut` directly and `advance` the buffer in place, so this
split / unsplit dance disappears; until then, every match arm in
`decode_responses` is required to handle the tail explicitly.

## Cross-cutting guidance for changes in `connection.rs`

1. **Diff against `may_postgres/src/connection.rs` first.** Any change
   to the loop structure, the `read_blocked` flag, the `io_events`
   bitmask, or the call order of `process_req` / `nonblock_write` /
   `nonblock_read` / decode / `wait_io` must be cross-checked against
   may_postgres. The may_postgres loop is battle-tested; if may-redis
   diverges from it, document **why** in code comments.
2. **Never drop the `bool` from `nonblock_read`.** It is the only signal
   that tells the loop whether to `wait_io()` or re-read immediately.
3. **Never drop bytes out of a `RESPReader` that came from
   `read_buf.split()`.** Either consume them or `unsplit` the remainder
   back.
4. **Add a multi-value test for every decoder change.** Single-value
   tests will not catch dispatch bugs that only appear when multiple
   RESP frames share one read.
5. **Run the full `client::client::tests::test_integration_*` suite
   under `--test-threads=1` after every connection-loop change.** They
   are the canonical end-to-end coverage and they hang (rather than
   fail loudly) when these classes of bugs regress.
