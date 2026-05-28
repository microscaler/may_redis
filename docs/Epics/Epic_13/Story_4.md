# Story 4 — Unsafe Blocks and State Corruption

**Finding IDs:** #6, #11, #20 (CRITICAL/HIGH/MEDIUM)

**Objective:** Eliminate unsafe code where possible, fix state corruption bugs, and add safety invariants to remaining unsafe blocks.

---

## Issue #6: Unsafe Blocks in nonblock_read/nonblock_write

**Severity:** CRITICAL

### Problem Description

Two unsafe blocks exist in the connection loop:

**nonblock_read (line 225):**
```rust
let buf: &mut [u8] = unsafe { &mut *(read_buf.chunk_mut() as *mut _ as *mut [u8]) };
```
This transmutes `BytesMut::ChunkMut` to `&mut [u8]` to write raw bytes into the buffer's capacity portion.

**nonblock_write (line 275):**
```rust
stream.write(unsafe { buf.get_unchecked(write_cnt..) })
```
This uses `get_unchecked` in a loop, relying on the invariant that `write_cnt <= buf.len()`.

Both are correct by `bytes` crate invariants, but if the loop invariants are ever broken by a logic bug, these cause undefined behavior.

### Attack Vector

These are not direct exploits but correctness bugs that become exploitable if a logic error is introduced. For example, if a future refactor changes the loop condition and `write_cnt` exceeds `buf.len()`, `get_unchecked` reads past the buffer boundary — potentially leaking sensitive data from adjacent memory or causing a crash.

### Acceptance Criteria

1. **AC-4.1:** The `get_unchecked` in `nonblock_write` must be replaced with safe bounds-checked indexing.
2. **AC-4.2:** The `chunk_mut` transmute in `nonblock_read` must be reviewed and documented with invariants.
3. **AC-4.3:** Both functions must have a doc comment listing every unsafe invariant.
4. **AC-4.4:** If replacement with safe code is not possible, `#[allow(clippy::undocumented_unsafe_blocks)]` must NOT be used — each unsafe block must be preceded by a comment explaining WHY unsafe is required.

### Functional Requirements

- **FR-033:** Replace `buf.get_unchecked(write_cnt..)` with `&buf[write_cnt..]` (safe bounds-checked slice).
- **FR-034:** Replace `read_buf.chunk_mut() as *mut _ as *mut [u8]` with `read_buf.chunk_mut().as_mut()` (the `bytes` crate provides safe access).
- **FR-035:** If safe replacement is not possible, document each unsafe invariant with `// SAFETY: ...` comments referencing the specific bytes crate guarantee.

### Non-Functional Requirements

- **NFR-017:** No new unsafe blocks may be introduced without written justification in a code comment.
- **NFR-018:** The existing unsafe code must be verified correct by code review by at least one other developer.

---

## Issue #11: RESPReader Depth Limit Uses Mutable State — Not Re-entrant

**Severity:** HIGH

### Problem Description

The depth counter in `RESPReader` is managed manually:
```rust
if self.depth >= self.max_depth { ... }
self.depth += 1;
// ... recursive read_value() ...
self.depth -= 1;  // ← If this line is skipped due to panic, depth is corrupted
```

If `read_value()` panics between `depth += 1` and `depth -= 1` (e.g., from OOM in `Vec::with_capacity(len)`), the depth counter is left incremented. Subsequent reads will incorrectly think they're at a deeper nesting level, potentially rejecting valid responses.

### Attack Vector

An attacker sends a deeply nested RESP array that triggers an OOM. The panic corrupts the depth counter. Subsequent legitimate responses (at normal depth) are incorrectly rejected as "too deep," causing a denial of service.

### Acceptance Criteria

1. **AC-4.5:** Depth increment/decrement must be RAII-guarded — if the scope exits abnormally, the counter is restored.
2. **AC-4.6:** The depth guard must be implemented without `unsafe` code.
3. **AC-4.7:** Existing tests for depth limits must continue to pass.
4. **AC-4.8:** The depth limit check must still occur before recursion (not after).

### Functional Requirements

- **FR-036:** Implement a `DepthGuard` RAII struct:
  ```rust
  struct DepthGuard<'a> {
      reader: &'a mut RESPReader,
  }
  impl Drop for DepthGuard {
      fn drop(&mut self) { self.reader.depth -= 1; }
  }
  ```
- **FR-037:** `read_array()` must create a `DepthGuard` at the start and return it. The caller drops the guard at the end.
- **FR-038:** The depth check must happen before creating the guard (so the guard never increments).

### Non-Functional Requirements

- **NFR-019:** The RAII guard must add zero runtime overhead — just a struct field and Drop impl.
- **NFR-020:** The guard must work correctly with `?` operator — if `?` propagates an error through the guard's scope, Drop still runs.

---

## Issue #20: unsafe { rx.cancel() } Has No Safety Invariants

**Severity:** MEDIUM

### Problem Description

```rust
unsafe { rx.cancel() };
```

The coroutine cancellation is inherently unsafe. If the connection loop is in the middle of dispatching a response when cancelled, the `PendingRequest` queue could be left in a corrupted state. The safety relies on the coroutine yielding at "safe points" — but there's no formal verification of those invariants.

### Attack Vector

An attacker who can trigger rapid connection drops (e.g., by connecting and immediately closing) while many coroutines are waiting on responses can corrupt the pending request queue. If the queue state is corrupted, subsequent responses might be dispatched to the wrong coroutine, causing data leakage between users.

### Acceptance Criteria

1. **AC-4.9:** The cancellation safety invariant must be documented in a `// SAFETY:` comment.
2. **AC-4.10:** If the invariant cannot be formally verified, implement a "graceful shutdown" path that drains the pending request queue before cancelling.
3. **AC-4.11:** All pending requests must receive an error response (not be silently dropped) when the connection is cancelled.
4. **AC-4.12:** The cancellation must not cause the connection loop's `resp_queue` to become corrupted — verify via test.

### Functional Requirements

- **FR-039:** Document the cancellation invariant: "The connection loop only yields at the following points: (a) after epoll waits, (b) between read/write cycles, (c) during spsc channel operations."
- **FR-040:** Implement `Connection::shutdown()` that (a) marks the connection as closed, (b) drains `resp_queue` with error responses, (c) then cancels the coroutine.
- **FR-041:** `Drop` must call `shutdown()` instead of directly calling `rx.cancel()`.

### Non-Functional Requirements

- **NFR-021:** Graceful shutdown must complete within 100ms of being called.
- **NFR-022:** Shutdown must not block the calling thread — it should return immediately and the draining happens in a separate coroutine.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/connection/connection.rs` lines 218-248: nonblock_read unsafe blocks
- `src/connection/connection.rs` lines 270-291: nonblock_write unsafe blocks
- `src/connection/connection.rs` lines 141-156: Drop/cancellation
- `src/codec/reader.rs` lines 249-285: depth counter management
