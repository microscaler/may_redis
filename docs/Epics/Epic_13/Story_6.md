# Story 6 — Connection Lifecycle and Response Integrity

**Finding IDs:** #13, #15 (MEDIUM)

**Objective:** Fix response integrity issues, implement rate limiting for auth attempts, and improve error visibility.

---

## Issue #13: decode_responses Silently Drops Unexpected Responses

**Severity:** MEDIUM

### Problem Description

```rust
} else {
    log::warn!("unexpected response from server");
}
```

When the server sends more responses than expected (e.g., pub/sub messages mixed into the response stream, or a Redis bug), responses are silently dropped. Only a warning is logged — the caller never sees the unexpected response.

### Attack Vector

If the connection loop is shared across multiple application coroutines and one coroutine unexpectedly receives a response intended for another, that coroutine's `rx.recv()` may return the wrong value. This is a data integrity issue: the application processes a response for a different command.

### Acceptance Criteria

1. **AC-6.1:** When `resp_queue` is empty but a response is decoded, the response must be logged with the full RedisValue contents (command name, not just "unexpected").
2. **AC-6.2:** If more than 10 unexpected responses occur in a 60-second window, the connection must be closed as "protocol violation."
3. **AC-6.3:** The connection loop must track the expected response count vs. actual response count per request batch.
4. **AC-6.4:** A metric counter (via `log::info!`) must be incremented for each unexpected response.

### Functional Requirements

- **FR-052:** Add a `last_expected_response_count: usize` field to the connection loop state.
- **FR-053:** Before each `process_req`, record the number of pending requests. After `decode_responses`, verify the count matches.
- **FR-054:** Implement a sliding window counter for unexpected responses (10 per 60s triggers disconnect).
- **FR-055:** Log the first 100 bytes of an unexpected response for debugging purposes.

### Non-Functional Requirements

- **NFR-029:** The unexpected response detection must add <= 1 microsecond of overhead per decode iteration.
- **NFR-030:** The sliding window counter must be O(1) — use a ring buffer or timestamp-based expiry.

---

## Issue #15: No Auth Brute-Force Protection

**Severity:** MEDIUM

### Problem Description

The `auth()` method sends passwords directly to Redis with no rate limiting:
```rust
fn auth(&self, password: &str) -> CommandBuilder {
    CommandBuilder::new("AUTH").arg(password)
}
```

There is no rate limiting on auth attempts. An attacker who can trigger auth calls can brute-force Redis passwords.

### Attack Vector

If this library is used in a web application where connection failures trigger re-authentication (common in connection pool configurations), an attacker can:
1. Flood the application with requests that fail auth
2. The connection pool re-authenticates on each failure
3. The attacker observes which passwords cause different error responses (timing or error message differences)
4. Gradually narrow down the correct password

### Acceptance Criteria

1. **AC-6.5:** The `Connection` must track failed auth attempts per client.
2. **AC-6.6:** After N consecutive failed auth attempts (default: 10), the connection must be closed.
3. **AC-6.7:** The auth attempt count must reset after a successful auth.
4. **AC-6.8:** The failed auth limit must be configurable per connection.
5. **AC-6.9:** The limit applies only to failed auth responses from Redis (WRONGPASS), not to network errors.

### Functional Requirements

- **FR-056:** Add `failed_auth_attempts: usize` to `Connection`.
- **FR-057:** After each `AUTH` command response, check if Redis returned `WRONGPASS`. If so, increment the counter.
- **FR-058:** If `failed_auth_attempts >= max_failed_auth` (default 10), disconnect and return `Err(RedisError::Auth("too many failed auth attempts"))`.
- **FR-059:** Reset `failed_auth_attempts` to 0 on successful auth (Redis returns `+OK`).
- **FR-060:** Add `Connection::connect_with_auth_limit()` constructor accepting `max_failed_auth: usize`.

### Non-Functional Requirements

- **NFR-031:** The auth attempt counter must not be observable by the application — the error message must be generic ("authentication failed"), not "attempt 5 of 10 failed."
- **NFR-032:** The counter must survive reconnection — if the application reconnects and fails auth again, the counter resets (it tracks per-connection, not per-application).

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/connection/connection.rs` lines 344-346: unexpected response silently dropped
- `src/protocol/commands.rs` lines 118-122: auth method with no rate limiting
