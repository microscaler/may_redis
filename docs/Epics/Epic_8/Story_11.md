# Story 8.11 — Fix InMemoryClient to Return Null for Missing Keys

**Objective:** Fix `InMemoryStore::get()` to return a `Null` bulk string representation instead of `Err(RedisError::Other(...))` for missing keys. Real Redis `GET` on a missing key returns `$-1\r\n` (null bulk string), not an error. This mismatch causes test-reality divergence.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1-8.7 (basic type conversions).

**Source docs:** `docs/redis-implementation-audit.md` (Finding #7, HIGH), `src/client/in_memory.rs` lines 37-42

## The Problem

```rust
pub fn get(&mut self, key: &str) -> Result<String, RedisError> {
    self.cleanup();
    match self.data.get(key) {
        Some((value, _)) => Ok(value.clone()),
        None => Err(RedisError::Other(format!("no such key: {key}"))),  // WRONG
    }
}
```

Real Redis `GET missing_key` returns `$-1\r\n` (null). The InMemoryClient returns an error instead.

**Impact on tests:**
- `client.execute::<Option<String>>(client.get("missing"))` → works (Null → None in Option) — but the InMemoryClient returns Error, not Null, so `Option<String>` actually fails! This means tests using `Option<String>` with InMemoryClient for missing keys are silently broken.
- `client.execute::<String>(client.get("missing"))` → InMemoryClient gives `Err(Other("no such key"))` while real Redis gives `Ok(RedisValue::Null)` which `String::from_redis_value` rejects with `Parse`. Different error types.

## Functional Requirements

1. `InMemoryStore::get()` must return a `RedisValue` instead of `Result<String, RedisError>` for missing keys, allowing the codec layer to handle the conversion.
2. **OR** — simpler approach: `InMemoryStore::get()` returns `Result<Option<String>, RedisError>` where `Ok(None)` represents Null, and callers convert `None` to `RedisValue::Null` before returning.
3. **Best approach:** Refactor InMemoryClient to return `RedisValue` directly (matching real Redis wire format), then let `execute<T>()` handle the conversion. This is the cleanest.

## Recommended Approach (Refactor to RedisValue)

1. `InMemoryStore::get()` → `Result<RedisValue, RedisError>`
   - Missing key → `Ok(RedisValue::Null)`
   - Found → `Ok(RedisValue::BulkString(value.into_bytes()))`
2. `InMemoryClient::get()` → `Result<RedisValue, RedisError>`
3. All other InMemoryClient methods similarly return `RedisValue`
4. `execute::<T>()` converts `RedisValue` via `T::from_redis_value()`

## Non-Functional Requirements

1. **Backwards compatible for test users** — InMemoryClient is only used under `feature = "test"`, so breaking changes here only affect tests.
2. **Wire-format accurate** — InMemoryClient responses must match what real Redis would send on the wire.
3. **No may dependency** — in_memory.rs has no `may` imports.

## Code Anchors

- `src/client/in_memory.rs` — `InMemoryStore::get()`, `InMemoryClient::get()`, all methods

## Tasks

1. Change `InMemoryStore::get()` to return `RedisValue`.
2. Change `InMemoryClient::get()` to return `RedisValue`.
3. Update all other InMemoryClient methods to return `RedisValue`.
4. Verify `execute::<Option<String>>(client.get("missing"))` returns `Ok(None)` via InMemoryClient.
5. Update all existing InMemoryClient tests.

## Unit Test Plan

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `missing_key_returns_null` | `InMemoryClient.get("missing")` | `RedisValue::Null` |
| `existing_key_returns_bulk` | `InMemoryClient.set("k","v")` then `get("k")` | `RedisValue::BulkString(b"v")` |
| `option_string_missing` | `execute::<Option<String>>(get("missing"))` | `Ok(None)` |
| `option_string_existing` | `execute::<Option<String>>(get("key"))` | `Ok(Some("val"))` |
| `string_missing_errors` | `execute::<String>(get("missing"))` | `Err(Parse(...))` (Null → String fails) |
| `tvl_missing_errors` | `InMemoryClient.ttl("missing")` | `Err(...)` (TTL on missing key is still an error) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass (InMemoryClient tests updated)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `execute::<Option<String>>(get("missing"))` → `Ok(None)`
- [ ] `execute::<String>(get("missing"))` → `Err(Parse(...))` (not `Err(Other(...))`)
- [ ] Error-returning methods (TTL missing, EXPIRE missing, DEL missing) still return errors correctly
