# Story 10.2 — Add # Errors Sections to All Result-Returning Items

**Objective:** Add `# Errors` sections to all 19 public interfaces returning `Result<T, E>` that were missing them.

**Epic:** 10 — Lint Tightening & Mandatory Rustdocs

**Dependencies:** Story 10.1 (lint tightening must be done first to trigger the errors)

**Status:** COMPLETE

**Source docs:** Clippy output from Story 10.1 showing which files were missing `# Errors`

## Code Anchors

- `src/client/client.rs` — 4 items: `execute_with_timeout`, `execute_timeout`, `execute`, `ping`
- `src/client/in_memory.rs` — 10 items: `InMemoryStore::get`, `InMemoryStore::incr`, `InMemoryStore::ttl`, `InMemoryClient::get`, `InMemoryClient::del`, `InMemoryClient::exists`, `InMemoryClient::incr`, `InMemoryClient::ttl`, `InMemoryClient::expire`, `InMemoryClient::keys`, `InMemoryClient::dbsize`
- `src/client/pipeline.rs` — 2 items: `FromPipelineResponse::from_responses`, `Pipeline::execute`
- `src/connection/tcp.rs` — 5 items: `TcpConnector::connect`, `connect_with_timeout`, `connect_timeout`, `connect_url`, `connect_url_timeout`
- `src/codec/reader.rs` — 1 item: `RESPReader::read_value`
- `src/core/error.rs` — 1 item: `FromRedisValue::from_redis_value`

## Changes Made

Added `# Errors` sections to each item, with descriptions of the specific error types returned:

### client/client.rs
- `execute_with_timeout` — `RedisError::Connection` on TCP failure, channel closed, or timeout
- `execute_timeout` — Same as `execute_with_timeout` (delegates to it)
- `execute` — Connection errors + `RedisError::Parse` on type conversion failure
- `ping` — `RedisError::Parse` on non-PONG response or connection failure

### client/in_memory.rs (InMemoryStore)
- `get` — Infallible (returns empty string for missing keys)
- `incr` — `RedisError::Other` if value is not a valid integer
- `ttl` — `RedisError::Other` if key does not exist

### client/in_memory.rs (InMemoryClient)
- All public methods — `RedisError::Parse` if the mutex is poisoned

### client/pipeline.rs
- `from_responses` — `RedisError::Parse` on count mismatch or type conversion failure
- `execute` — Delegates to `execute_raw` + `from_responses`, both can return `RedisError`

### connection/tcp.rs
- `connect` — `ConnectionError` on DNS, TCP, nodelay, or timeout failure
- `connect_with_timeout` — Same with specific `ConnectionError` variants
- `connect_timeout` — Convenience wrapper, same errors as `connect_with_timeout`
- `connect_url` — `ConnectionError::Connect` on malformed URL or invalid port
- `connect_url_timeout` — Same + timeout

### codec/reader.rs
- `read_value` — `RedisError::Parse` on malformed wire format: missing CRLF, unknown marker, incomplete bulk string, invalid integer length, or array length exceeding configured limits

### core/error.rs
- `FromRedisValue::from_redis_value` — `RedisError::Parse` if the `RedisValue` does not match the expected type for the target Rust type

## Verification

- `cargo clippy --lib --all-features` — zero `missing_errors_doc` errors
- `cargo clippy --lib --tests --all-features` — zero `missing_errors_doc` errors
- `cargo test --lib --all-features` — 341 passed, 0 failed, 28 ignored
