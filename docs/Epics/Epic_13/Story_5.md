# Story 5 — Memory and Resource Limits

**Finding IDs:** #10, #12, #19 (HIGH/MEDIUM/LOW)

**Objective:** Set reasonable default memory limits, fix InMemoryClient semantics, and protect against response-based OOM.

---

## Issue #10: InMemoryClient::get() Returns "" Instead of Null for Missing Keys

**Severity:** HIGH

### Problem Description

```rust
pub fn get(&mut self, key: &str) -> Result<String, RedisError> {
    match self.data.get(key) {
        Some((value, _)) => Ok(value.clone()),
        None => Ok(String::new()),  // ← Returns empty string, NOT Null
    }
}
```

Real Redis returns `NULL` (bulk string `$-1\r\n`) for missing keys. `InMemoryClient` returns `Ok("")` — an empty bulk string (`$0\r\n\r\n`).

When deserialized via `FromRedisValue` for `Option<String>`:
- Real Redis `NULL` → `None`
- InMemoryClient `""` → `Some("")`

Tests using `InMemoryClient` pass but would fail with real Redis.

### Attack Vector

This is primarily a correctness bug, not a security exploit. However, in a testing context where `InMemoryClient` is used as a test double, bugs that pass in tests but fail with real Redis can lead to production outages. An attacker who discovers that the application behaves differently with real Redis (e.g., checks `Option` for None vs Some("")) can exploit the difference.

### Acceptance Criteria

1. **AC-5.1:** `InMemoryClient::get()` for a missing key must return `Ok("")` where `""` represents `Null` in RESP semantics.
2. **AC-5.2:** The `InMemoryClient` must distinguish between a stored empty string and a missing key internally.
3. **AC-5.3:** When `FromRedisValue::from_redis_value` is called on the `InMemoryClient` response, `Option<String>` must return `None` for missing keys.
4. **AC-5.4:** Existing `InMemoryClient` tests must be updated to reflect the correct semantics.

### Functional Requirements

- **FR-042:** `InMemoryStore::get()` must return `Result<Option<String>, RedisError>` — `Ok(Some(v))` for found, `Ok(None)` for missing.
- **FR-043:** `InMemoryClient::get()` must convert `Ok(None)` to a response that `FromRedisValue` treats as `Null`.
- **FR-044:** The `FromRedisValue` impl for `Option<String>` must recognize `InMemoryClient`'s missing-key response as `Null`.

### Non-Functional Requirements

- **NFR-023:** The change must not break existing `InMemoryClient` users — the public API surface must remain compatible.
- **NFR-024:** `InMemoryClient` performance must not degrade — no additional allocations for the missing-key case.

---

## Issue #12: Large Bulk String Default Limit Is 256 MB

**Severity:** MEDIUM

### Problem Description

```rust
const DEFAULT_MAX_BULK_LEN: usize = 268_435_456; // 256 MB
```

A malicious RESP response could send many 256MB bulk strings. Combined with `DEFAULT_MAX_ARRAY_LEN` of 1 million, a single response could allocate 256 TB of memory before being rejected.

### Attack Vector

An attacker who can control the Redis server (or a MITM attacker who can inject RESP responses) can send a single bulk string of 256MB. If the application processes multiple such responses, total memory usage exceeds available RAM, causing OOM and process death.

### Acceptance Criteria

1. **AC-5.5:** The default maximum bulk string length must be reduced to 1 MB (1,048,576 bytes).
2. **AC-5.6:** The default maximum array length must be reduced to 10,000 elements.
3. **AC-5.7:** The default maximum array nesting depth must remain at 256 (reasonable for most use cases).
4. **AC-5.8:** All three limits must be configurable via `RESPReader::with_max_*()` methods.
5. **AC-5.9:** A `RESPReader::secure()` builder must exist that sets conservative defaults (1MB bulk, 10K array, 64 depth).

### Functional Requirements

- **FR-045:** Change `DEFAULT_MAX_BULK_LEN` to `1_048_576` (1 MB).
- **FR-046:** Change `DEFAULT_MAX_ARRAY_LEN` to `10_000`.
- **FR-047:** Add `RESPReader::secure() -> Self` method that sets: max_bulk=1MB, max_array=10K, max_depth=64.
- **FR-048:** Document the default limits in the `RESPReader` doc comments.

### Non-Functional Requirements

- **NFR-025:** Reducing the default limits must not break any existing application that relies on the current defaults. If it does, the application must be able to opt back in via `with_max_bulk_len()`.
- **NFR-026:** The limits must be checked before memory allocation — not after. The reader must check the bulk length header value before allocating the `Vec<u8>`.

---

## Issue #19: No Response Size Limit on the Read Buffer

**Severity:** LOW

### Problem Description

```rust
let mut read_buf = BytesMut::with_capacity(65536);
```

The read buffer starts at 64KB but grows unbounded. Combined with the bulk string limit, a malicious server could stream large responses that fill the buffer and cause OOM.

### Attack Vector

An attacker who controls the Redis server (or a proxy) can send a stream of bulk strings that slowly fill the read buffer. Even with a per-bulk limit, if the application is slow to process responses, the buffer accumulates unconsumed data. Over time, this exhausts memory.

### Acceptance Criteria

1. **AC-5.10:** The connection loop's read buffer must have a configurable maximum size (default: 64MB).
2. **AC-5.11:** When the read buffer exceeds the maximum, the connection must be closed with an error.
3. **AC-5.12:** The buffer must be truncated (not grow beyond the limit) — not reset, so partial responses are preserved.

### Functional Requirements

- **FR-049:** Add `max_read_buffer: usize` to `spawn_connection_loop` parameters.
- **FR-050:** After each `nonblock_read`, check `read_buf.len()` against the limit. If exceeded, log an error and break the loop.
- **FR-051:** The limit must be configurable via `Connection::connect_with_buffer_limit()`.

### Non-Functional Requirements

- **NFR-027:** The buffer limit check must be O(1) — just a comparison, no allocation.
- **NFR-028:** The default 64MB must be large enough for most legitimate responses but small enough to prevent OOM.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/client/in_memory.rs` lines 42-48: InMemoryClient::get() returning "" instead of Null
- `src/codec/reader.rs` lines 8-15: DEFAULT_MAX_BULK_LEN, DEFAULT_MAX_ARRAY_LEN, DEFAULT_MAX_DEPTH
- `src/connection/connection.rs` line 412: read_buf with 64KB initial capacity, unbounded growth
