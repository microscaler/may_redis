# Story 7 — Pipeline and InMemoryClient Fixes

**Finding IDs:** #14, #17 (MEDIUM/LOW)

**Objective:** Fix pipeline error handling, InMemoryClient Null semantics, and ping() validation.

---

## Issue #14: execute_raw_results Silently Swallows Channel Errors

**Severity:** MEDIUM

### Problem Description

```rust
// pipeline.rs lines 145-149
if let Ok(val) = receivers[i].try_recv() {
    results[i] = Some(Ok(val));
    done += 1;
}
// Err(RecvError) is silently ignored — slot stays None
```

When a connection is dropped, the spsc channel is closed. `try_recv()` returns `Err(RecvError)` and the slot stays `None`. The loop spins on `yield_now()` waiting for a result that will never arrive. This causes the pipeline to **hang indefinitely**.

Additionally, when the loop does complete (via some other mechanism), the `results` vector contains `None` entries that are filtered out by `flatten()`, silently dropping failed commands from the pipeline result.

### Attack Vector

An attacker who can cause connection drops (network partition, Redis crash, firewall rule) during a pipeline execution can cause the client to hang indefinitely. This is a denial of service — the application's coroutine pool fills up with hung pipelines, and no new requests can be processed.

### Acceptance Criteria

1. **AC-7.1:** When a channel returns `RecvError`, the pipeline must surface this as `Err(RedisError)` in the results, not silently skip the slot.
2. **AC-7.2:** `execute_raw_results` must have a configurable timeout (default: 30 seconds) so it cannot hang indefinitely.
3. **AC-7.3:** The timeout must be configurable via a new method on `Pipeline`.
4. **AC-7.4:** Partial results must be returned — if 3 of 5 commands succeed and 2 fail, return 3 Ok values and 2 Err values.

### Functional Requirements

- **FR-061:** In the `try_recv` loop, change `if let Ok(val) = ...` to `match receivers[i].try_recv()` that handles both `Ok(val)` and `Err(RecvError)`.
- **FR-062:** On `Err(RecvError)`, set `results[i] = Some(Err(RedisError::Connection("response channel closed".into())))` and increment `done`.
- **FR-063:** Add `Pipeline::execute_raw_results_timeout(timeout: Duration) -> Vec<Result<RedisValue, RedisError>>` with a timeout.
- **FR-064:** If the timeout expires before all results are received, return partial results for completed commands and `Err(RedisError::Connection("pipeline timed out".into()))` for remaining slots.
- **FR-065:** Keep the existing `execute_raw_results()` with an infinite wait for backward compatibility (but deprecate it).

### Non-Functional Requirements

- **NFR-033:** The timeout implementation must not use blocking sleep — use `may::timer::sleep` for coroutine-native timing.
- **NFR-034:** Partial results must preserve order — slot 0 in the result corresponds to command 0 in the pipeline.

---

## Issue #17: ping() Rejects Custom PONG Messages

**Severity:** LOW

### Problem Description

```rust
// client.rs lines 261-266
pub fn ping(&self) -> Result<String, RedisError> {
    let cmd = CommandBuilder::new("PING");
    let response = self.execute::<String>(cmd)?;
    if response == "PONG" {
        Ok(response)
    } else {
        Err(RedisError::Parse(format!(
            "unexpected PING response: {response}"
        )))
    }
}
```

Redis supports custom PING messages: `PING hello` returns `hello`. The `ping()` method rejects any response that isn't literally "PONG". This breaks compatibility with Redis's PING/PONG protocol.

Additionally, Redis 6.2+ supports `PING` returning the server's ping reply, which may be customized by the application.

### Attack Vector

This is not a security exploit but a compatibility bug. An application that depends on `ping()` to verify connectivity will fail when connecting to a Redis instance that returns a custom PONG, or when the Redis server has been configured with a custom PONG message.

### Acceptance Criteria

1. **AC-7.5:** `ping()` must accept any non-error response from Redis, not just "PONG".
2. **AC-7.6:** The method must return the actual response string (not hardcoded "PONG").
3. **AC-7.7:** An `assert_ping_ok()` method must exist that specifically checks for "PONG" (for applications that need strict verification).
4. **AC-7.8:** Existing code that depends on `ping()` returning exactly "PONG" must continue to work (the default Redis behavior returns "PONG").

### Functional Requirements

- **FR-066:** Change `ping()` to return the raw response: `Ok(response)` for any `String` response, `Err` for protocol errors.
- **FR-067:** Add `RedisClient::assert_ping_ok()` that calls `ping()` and checks the response is "PONG".
- **FR-068:** The `assert_ping_ok()` method must return `Result<(), RedisError>` with a descriptive error message.
- **FR-069:** Update all existing usages of `ping()` in tests to use the new method if they depend on "PONG".

### Non-Functional Requirements

- **NFR-035:** The change must not break any existing application — the default Redis behavior (returning "PONG") must work exactly as before.
- **NFR-036:** `ping()` latency must not increase — it's a single round-trip command.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/client/pipeline.rs` lines 144-158: execute_raw_results silently swallowing errors
- `src/client/client.rs` lines 258-268: ping() rejecting non-"PONG" responses
- `src/client/in_memory.rs` lines 42-48: InMemoryClient returning "" instead of Null (also in Story 5)
