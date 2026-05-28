# May-Redis Code Review — 2026-05-28

> **Reviewer:** Expert review (may coroutines + Redis API)
> **Branch:** main
> **Scope:** Full codebase — architecture, correctness, safety, performance, API fidelity
> **Build status:** ✅ `cargo build` clean, 313 tests pass, clippy --lib --tests --all-features clean
> **Known issue:** `examples/debug_redis.rs` uses `unwrap()` which violates clippy deny rules (not production code)

---

## Executive Summary

may-redis is a well-structured, coroutine-native Redis client built on the `may` runtime. The codebase demonstrates strong adherence to its stated principles: zero tokio, zero async/await, pure may coroutines. The connection loop correctly mirrors may_postgres patterns. The RESP2 codec is complete and hardened. The command surface covers 80+ Redis commands across all major categories.

**Overall assessment: GOOD — production-viable for the scoped use case (sesame-idam), with specific areas requiring attention before broader deployment.**

---

## 1. Architecture & Module Structure

### Strengths

| Area | Assessment |
|------|-----------|
| Module separation | Clean 5-module hierarchy (core → codec → protocol → connection → client) |
| Dependency direction | core/codec have zero may dependency — correct isolation |
| Single-crate approach | ADR-001 documents the rationale; pragmatic for current scale |
| Re-exports | `src/lib.rs` provides ergonomic top-level API |

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| A1 | INFO | Target workspace split (6 crates) is documented but not implemented. Current single-crate is appropriate for the project's maturity. | `docs/08-module-structure.md` |
| A2 | LOW | `src/connection/connection.rs` has 6 `#![allow(...)]` attributes that duplicate those in `src/lib.rs`. These inner-module allows could be removed since the crate-level allows already cover them. | `src/connection/connection.rs:38-43` |
| A3 | INFO | `src/connection/epoll.rs` is declared in directory listing but unused (no `pub mod epoll` in `connection/mod.rs`). Dead file. | `src/connection/` |

---

## 2. Connection Loop (Most Critical Code)

### Strengths

- Correctly mirrors `may_postgres` connection loop pattern
- Bug 1 (busy-spin) and Bug 2 (dropped pipeline bytes) are **both fixed** with comprehensive regression tests
- Extensive doc comments explaining invariants, especially around `nonblock_read` return value semantics
- `WaitIoWaker` correctly signals the loop on new requests
- Error propagation drains all pending senders on fatal I/O errors

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| C1 | MEDIUM | `nonblock_read` uses `unsafe` to get mutable access to `BytesMut` chunk via raw pointer cast. While this follows `may_postgres`, the safety invariant (buffer is initialized up to `read_cnt`) should be documented with a `// SAFETY:` comment per Rust convention. | `connection.rs:200-211` |
| C2 | MEDIUM | `nonblock_write` similarly uses `unsafe` for unchecked slice indexing. The bounds are guaranteed by the `while read_cnt < len` loop, but the `// SAFETY:` comment is missing. | `connection.rs:240` |
| C3 | LOW | `Connection::drop` uses `unsafe { rx.cancel() }`. This is the may coroutine cancellation API, but it could leave partial state if the loop is mid-write. Consider adding a comment about the safety contract (may guarantees cancellation at yield points). | `connection.rs:143-145` |
| C4 | LOW | `process_req` reserves 65536 bytes when capacity drops below 512. The magic numbers should be named constants for maintainability. | `connection.rs:163-166` |
| C5 | INFO | The `io_events & 1` bitmask check (line 401) relies on may's internal event representation. This is correct per may_postgres, but fragile if may changes its event encoding. | `connection.rs:401` |

---

## 3. RESP2 Codec

### Strengths

- RESPWriter: complete, correct encoding for all 6 RESP2 types
- RESPReader: configurable depth/length/bulk caps prevent OOM/stack overflow
- Strict CRLF enforcement
- Comprehensive roundtrip test suite (26 tests including edge cases)

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| R1 | LOW | `RESPReader::skip_crlf()` is called at the start of `read_value()`. If the buffer starts with stray CRLF bytes (e.g., from a protocol-violating proxy), they are silently consumed. This is defensively correct but could mask upstream issues. | `codec/reader.rs:88` |
| R2 | INFO | Default max bulk length is 256 MB. For a client library this is generous; most Redis deployments limit values to 512 MB. The cap is configurable, so this is appropriate. | `codec/reader.rs:9` |
| R3 | INFO | `itoa` crate is used for integer formatting in RESPWriter — correct and efficient choice (zero-allocation). | `codec/writer.rs:39-40` |

---

## 4. Type System (core/)

### Strengths

- `RedisValue` enum covers all RESP2 types with correct `#[derive]` traits
- `FromRedisValue` implementations are comprehensive: i64, String, bool, (), Vec<String>, Vec<i64>, Vec<RedisValue>, Option<String>, usize
- `ToRedisArgs` covers primitives (String, &str, i64, u32, f64, &[u8], bool, ()) plus blanket `&T` and `Vec<T>`
- Error types are properly segmented (Connection, Protocol, Parse, Other)

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| T1 | LOW | `FromRedisValue for usize` casts `i64` to `usize` with `*n as Self`. On 32-bit platforms, this could silently truncate values > 2^32. The check `*n >= 0` is present but no upper-bound check exists. | `core/from_value.rs:70` |
| T2 | INFO | No `FromRedisValue` impl for `u64`, `i32`, `u8`, or `f64`. These may be needed as the command surface expands (e.g., INCRBYFLOAT). | `core/from_value.rs` |
| T3 | INFO | `f64::to_string()` for `ToRedisArgs` has special handling for NaN/Inf/whole numbers. Redis INCRBYFLOAT accepts these, but some commands don't. The behavior is correct for the general case. | `core/to_args.rs:99-124` |

---

## 5. Command Surface (protocol/)

### Strengths

- 80+ commands implemented across all major Redis categories
- Consistent `#[must_use]` annotations on all builder methods
- Commands trait has default implementations — only `RedisClient` needs to override
- RESP wire encoding verified by per-command unit tests

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| P1 | MEDIUM | `mget`, `mset`, `msetnx`, `sinter`, `sunion` are associated functions (no `&self`) while all other commands take `&self`. This inconsistency means they can't be called on a `RedisClient` instance via the `Commands` trait in the normal way (`client.mget(...)` won't compile). | `protocol/commands.rs:186-212, 391-406` |
| P2 | LOW | `Commands` trait re-implements every method identically in the `impl Commands for RedisClient` block (lines 208+). This is redundant since the trait has default implementations. The `impl` block could simply be `impl Commands for RedisClient {}`. | `client/client.rs:208-250+` |
| P3 | INFO | SUBSCRIBE/UNSUBSCRIBE/PSUBSCRIBE/PUNSUBSCRIBE build command bytes but the connection loop doesn't implement the pub/sub state machine (switching from request-response to push mode). These commands will produce correct wire format but won't work correctly with the current connection loop. | `protocol/commands.rs` |
| P4 | INFO | BLPOP/BRPOP timeout handling: these blocking commands may exceed the client's 30-second default timeout. Users must use `execute_with_timeout` with an appropriate duration. No documentation warns about this. | `protocol/commands.rs` |

---

## 6. Client Layer

### Strengths

- `RedisClient` is `Clone` via `Arc<InnerClient>` — correct pattern for sharing across coroutines
- Pipeline implementation correctly yields between batch send and response collection
- Timeout uses coroutine-friendly `yield_now()` polling loop (not thread sleep on main coroutine)
- `InMemoryClient` provides clean test isolation without Redis server

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| CL1 | MEDIUM | `execute_with_timeout` spawns a timeout coroutine using `std::thread::sleep` inside `go!`. This blocks a may worker thread. Should use `may::timer::sleep` or `may::coroutine::sleep` instead. On a system with few may workers this could starve the scheduler. | `client/client.rs:103-106` |
| CL2 | LOW | The busy-poll loop in `execute_with_timeout` (try_recv → try_recv → yield_now) is a spin-wait pattern. Under load, this generates unnecessary context switches. A blocking `rx.recv()` with a cancellation channel would be more efficient, though the current approach works correctly. | `client/client.rs:111-124` |
| CL3 | LOW | `Pipeline::execute_raw_results` polls receivers with `try_recv` in a round-robin loop. For large pipelines (1000+ commands) this is O(n²) in the worst case. Acceptable for typical use but worth noting. | `client/pipeline.rs:142-158` |
| CL4 | INFO | `connect_url` strips "redis://" prefix but doesn't handle `rediss://` (TLS) or authentication in the URL (`******host:port/db`). Feature gap for future consideration. | `client/client.rs:55` |

---

## 7. Safety & Correctness

### Strengths

- `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` all denied in production code
- JSF-AV compliance audited (5/6 rules pass, 1 partial)
- No `unsafe` in core/, codec/, protocol/, or client/
- Only 3 `unsafe` blocks in connection/ — all following may_postgres patterns

### Findings

| # | Severity | Finding | Location |
|---|----------|---------|----------|
| S1 | MEDIUM | 3 `unsafe` blocks in `connection.rs` lack `// SAFETY:` comments (see C1, C2, C3). Rust best practice requires these. | `connection.rs` |
| S2 | LOW | `Connection::drop` cancels the coroutine but doesn't ensure in-flight requests receive error responses before cancellation. May's cancel semantics guarantee the coroutine will stop at its next yield point, but any request already dequeued but not yet sent over TCP will be silently dropped. | `connection.rs:141-145` |
| S3 | INFO | The `examples/debug_redis.rs` file violates clippy deny rules (uses `unwrap()`). Should either be excluded from clippy via `[[example]]` config or fixed with proper error handling. | `examples/debug_redis.rs` |

---

## 8. Performance Considerations

| Area | Assessment |
|------|-----------|
| Buffer allocation | Pre-allocated 64KB read/write buffers in connection loop ✅ |
| RESP encoding | Zero-allocation integer formatting via `itoa` ✅ |
| Command builder | Reuses internal `buf: Vec<Vec<u8>>` across `arg()` calls (JSF-AV Story 9.2) ✅ |
| Response dispatch | FIFO VecDeque — O(1) push/pop ✅ |
| Pipeline | Batch send + single yield + sequential collect ✅ |
| TCP | TCP_NODELAY set on connect (reduces latency for small commands) ✅ |

### Potential improvements (not bugs):

- Connection pooling is not implemented (single connection per `RedisClient`)
- No automatic reconnection on connection drop
- No DNS caching across reconnects
- No command batching/coalescing at the connection layer

---

## 9. Test Coverage

| Module | Unit Tests | Integration Tests | Coverage Assessment |
|--------|-----------|-------------------|---------------------|
| core/ | 45 | — | Excellent: all FromRedisValue paths, edge cases |
| codec/ | 52 | — | Excellent: roundtrip, CRLF strictness, caps |
| protocol/ | 120+ | — | Excellent: per-command RESP encoding verification |
| connection/ | 18 | 28 (ignored w/o Redis) | Good: decode_responses, process_req, TCP |
| client/ | 20 | 30+ (ignored w/o Redis) | Good: pipeline, timeout, InMemoryClient |

**Total: 313 passing, 28 ignored (require live Redis)**

---

## 10. Recommendations (Priority Order)

### Must-Fix (before production deployment)

1. **CL1:** Replace `std::thread::sleep` with `may::timer::sleep` in timeout coroutine to avoid blocking a may worker thread.
2. **S1/C1/C2:** Add `// SAFETY:` comments to all 3 `unsafe` blocks explaining the invariants.

### Should-Fix (improve quality)

3. **P1:** Make `mget`, `mset`, `msetnx`, `sinter`, `sunion` take `&self` for API consistency, or document why they don't.
4. **P2:** Remove redundant `impl Commands for RedisClient` method bodies — use default trait impls.
5. **A3:** Remove dead `src/connection/epoll.rs` file if unused.
6. **S3:** Fix or exclude `examples/debug_redis.rs` from clippy checks.

### Nice-to-Have (future work)

7. **P3:** Document that pub/sub commands produce correct wire format but require a dedicated connection (pub/sub state machine not implemented in connection loop).
8. **P4:** Document timeout considerations for blocking commands (BLPOP/BRPOP).
9. **CL4:** Consider redis:// URL parsing for auth, db selection, TLS.
10. **T1:** Add upper-bound check for `usize` conversion on 32-bit platforms.

---

## Conclusion

The may-redis codebase is **well-engineered** for its stated purpose. The connection loop — the most critical and subtle component — is correctly implemented with documented pitfalls and regression tests. The RESP2 codec is complete and hardened. The command surface covers all sesame-idam requirements and extends well beyond them.

The two must-fix items (worker thread blocking in timeout, missing safety comments) are the only findings that could cause production issues. Everything else is quality improvement or future feature work.

**Verdict: Approve with conditions (address CL1 and S1 before production use).**
