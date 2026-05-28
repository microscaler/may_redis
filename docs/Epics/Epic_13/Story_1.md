# Story 1 — Timeout Safety

**Finding IDs:** #1, #2, #16 (CRITICAL/MEDIUM)

**Objective:** Fix `execute_with_timeout` so that timeout cancellation actually prevents in-flight writes, eliminate unbounded coroutine spawn, and make timeout behavior explicit and safe.

---

## Issue #1: Timeout Does NOT Cancel In-Flight Requests — Silent Write Execution

**Severity:** CRITICAL

### Problem Description

`execute_with_timeout` sends the request to the connection loop at line 170 (`self.inner.connection.send(request)`) BEFORE starting the timeout coroutine. When the timeout fires and returns `Err(...)`, the command has **already been queued and sent to Redis**. The command executes on the server but the caller never sees the result.

For read commands (GET, EXISTS), this is a correctness bug. For write commands (SET, DEL, INCR, FLUSHDB), the command **succeeds on the server** while the caller gets a timeout error. This breaks "transactional" semantics.

**Timeline:**
```
T0: execute_with_timeout(cmd, 1ms) called
T0: Request sent to connection loop (already in Redis queue)
T1: Timeout fires, returns Err
T2: Redis processes the command (SET/DEL/INCR succeeds)
Caller: thinks it failed, continues without the result
```

### Attack Vector

An attacker who can induce timeouts (e.g., by running slow Redis, network partition, or SYN flood to the Redis port) causes writes to execute silently. In a caching layer, this creates inconsistency: the application thinks a SET failed and re-sends, but the server already has the value, leading to duplicate processing. In a session store, this could cause session data to be partially written.

### Root Cause

The request is sent to the connection loop before the timeout check. There is no cancellation mechanism to abort a pending request mid-flight.

### Acceptance Criteria

1. **AC-1.1:** Requests must NOT be sent to the connection loop until after a successful timeout check. The timeout must guard the send, not poll after send.
2. **AC-1.2:** A timed-out request must be fully cancelled — not just return an error, but ensure the RESP bytes are never written to the socket.
3. **AC-1.3:** Existing tests must continue to pass. New tests must verify that a timed-out SET command is NOT present in Redis (verified by GET returning null).
4. **AC-1.4:** The timeout mechanism must not introduce deadlock or livelock in the connection loop.

### Functional Requirements

- **FR-001:** Implement a pre-send timeout: the coroutine must check the timeout before sending, not poll after sending.
- **FR-002:** Implement request cancellation: when a timeout fires, the connection loop must discard the request's RESP bytes without writing them to the socket.
- **FR-003:** The `execute_with_timeout` API must preserve its signature — callers must not need to change code.
- **FR-004:** A `cancel()` method must be added to `Connection` that marks a specific tag as cancelled. When the connection loop processes a cancelled tag, it drops the pending request without sending.

### Non-Functional Requirements

- **NFR-001:** Timeout cancellation must add <= 1 microsecond of latency per request under normal (non-timeout) conditions.
- **NFR-002:** Cancellation must be thread-safe — multiple coroutines may timeout simultaneously.
- **NFR-003:** Cancellation must not cause the connection loop to hang on dropped channels.

---

## Issue #2: Timeout Coroutine Spawn Is Unbounded — Resource Exhaustion DoS

**Severity:** CRITICAL

### Problem Description

Every call to `execute_with_timeout` spawns a new `go!` coroutine for the timeout:

```rust
go!(move || {
    may::coroutine::sleep(timeout);
    let _ = timeout_tx.send(());
});
```

These coroutines are never cancelled or tracked. On timeout, the function returns but the sleeping coroutine continues until it fires. On success, the timeout coroutine is still running (its channel receiver is dropped but the coroutine isn't cancelled).

An attacker who can trigger many timeouts can spawn thousands of sleeping coroutines, exhausting the may scheduler's coroutine pool and causing all other coroutines to starve.

### Attack Vector

An attacker who can force the client to connect and issue commands with short timeouts (e.g., a web app endpoint that accepts user-controlled timeouts) can trigger coroutine exhaustion. Each sleeping coroutine consumes stack memory and scheduler state. After N spawns (where N = scheduler limit), new coroutines fail, causing a denial of service.

### Root Cause

The timeout coroutine is fire-and-forget with no way to cancel it when the response arrives first or the caller gives up.

### Acceptance Criteria

1. **AC-2.1:** There must be at most ONE sleeping timeout coroutine per request. When the response arrives, the timeout coroutine must be cancelled.
2. **AC-2.2:** When a timeout fires, the coroutine must be cleaned up — no leaked channels or stack.
3. **AC-2.3:** Running 10,000 concurrent timed-out requests must not exhaust the may scheduler (verified by integration test).
4. **AC-2.4:** The timeout mechanism must not leak memory — verified by heap profiling over 100K timed-out requests.

### Functional Requirements

- **FR-005:** Replace fire-and-forget `go!` with a tracked timeout handle. The handle must include a `cancel()` method that the caller invokes when the response is received.
- **FR-006:** The timeout handle must be `Drop`-safe — if the caller forgets to cancel, the timeout coroutine is cancelled on drop.
- **FR-007:** Implement a per-client timeout coroutine pool: reuse a single coroutine per client, resetting the sleep timer on each request.

### Non-Functional Requirements

- **NFR-004:** Coroutine pool must not add more than 5 microseconds of latency per request.
- **NFR-005:** The timeout pool must be bounded — at most one timeout coroutine per active request, but the total must not exceed the client's concurrent request limit.

---

## Issue #16: execute_timeout Default Is 30 Seconds — Silent Command Execution

**Severity:** MEDIUM

### Problem Description

The default `execute()` uses `Duration::from_secs(30)`. A command like `KEYS *` on a dataset with millions of keys will execute fully for 30 seconds before timing out. Since the command is already sent to the connection loop before the timeout starts, the command continues executing on the server even after the client gives up.

### Attack Vector

An attacker who can control the command pattern (e.g., through user input to a KEYS-like wildcard) can cause long-running server-side operations that waste Redis CPU and memory, even after the client has moved on.

### Acceptance Criteria

1. **AC-16.1:** The default timeout must be configurable via a `RedisClient::with_default_timeout()` builder method.
2. **AC-16.2:** The library must provide a safe default (e.g., 5 seconds) that is documented as the maximum expected command duration.
3. **AC-16.3:** A `connect_with_default_timeout()` constructor must accept a `Duration` parameter.
4. **AC-16.4:** The 30-second default must be documented as a security concern in the public API docs.

### Functional Requirements

- **FR-008:** Add a `default_timeout: Duration` field to `InnerClient`. `execute()` uses this field instead of hardcoded 30s.
- **FR-009:** The `RedisClient::connect()` method must accept an optional `Duration` argument (or a builder pattern) to set the default timeout.

### Non-Functional Requirements

- **NFR-006:** The default timeout field must not increase the size of `RedisClient` by more than 8 bytes (one `Duration` = 8 bytes).

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/client/client.rs` lines 157-210: `execute_with_timeout`
- `src/client/client.rs` lines 178-181: timeout coroutine spawn
- `src/connection/connection.rs` lines 176-189: `process_req` and request queue
