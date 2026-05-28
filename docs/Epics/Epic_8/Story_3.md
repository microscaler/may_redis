# Story 8.3 — Connection Timeout

**Objective:** Add a configurable timeout to TCP connection establishment. Currently, `TcpConnector::connect()` uses may::net::TcpStream::connect() which, while cooperative in may's coroutine context, has no explicit timeout. DNS resolution (`std::net::ToSocketAddrs`) and SYN handshakes can block indefinitely (75s default on Linux).

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Story 8.1 (FromRedisValue), Story 8.2 (ToRedisArgs) — no shared files, but needed for full epic completion.

**Source docs:** `docs/redis-implementation-audit.md` (Finding #3, critical), `docs/Epics/Epic_0/Story_0.md`

## Functional Requirements

1. `RedisClient::connect_with_timeout(host, port, timeout)` — new constructor accepting a `std::time::Duration` timeout. Uses `may::timer::sleep()` to enforce the limit. If timeout fires before connection completes, returns `ConnectionError::Timeout`.
2. `RedisClient::connect_timeout(host, port, seconds: u32)` — convenience method: converts seconds to Duration and calls `connect_with_timeout`. Default is 5 seconds.
3. `RedisClient::connect(host, port)` — existing method continues to work but uses a 5-second default timeout.
4. The timeout applies only to the TCP connect phase (DNS resolution + SYN + SYN-ACK). It does NOT affect command execution timeout (that is a future epic).
5. `connect_url(url, timeout)` — new method accepting timeout for URL-based connections.
6. `connect_url_timeout(url, seconds: u32)` — convenience method.

## Non-Functional Requirements

1. **Timeout must not hang the may scheduler** — Use `may::timer::sleep()` in a spawned coroutine that races against the connect. If the connect completes first, cancel the timer coroutine. If the timer fires first, abort the connect attempt.
2. **No extra dependencies** — Use only `may::timer::sleep` (already available via `may` crate).
3. **Error type** — Add `Timeout(String)` variant to `ConnectionError`.
4. **Backwards compatible** — Existing `connect()` and `connect_url()` signatures unchanged. They get the 5-second default.
5. **Zero may dependency in protocol** — Timeout logic lives in `connection/`, not `protocol/`.

## Code Anchors

- `src/connection/tcp.rs` — `ConnectionError::Timeout` variant, `TcpConnector::connect_timeout()`
- `src/client/client.rs` — `connect_with_timeout()`, `connect_timeout()`, `connect_url_timeout()` methods

## Tasks

1. Add `Timeout(String)` variant to `ConnectionError` in `tcp.rs`
2. Implement `TcpConnector::connect_with_timeout(host, port, timeout)` — races `may::timer::sleep` against `TcpStream::connect`
3. Implement `TcpConnector::connect_timeout(host, port, seconds: u32)` — delegates to connect_with_timeout with Duration::from_secs
4. Update `TcpConnector::connect_url` to accept timeout (or add `_timeout` variant)
5. Add `RedisClient::connect_with_timeout`, `connect_timeout`, `connect_url_timeout`
6. Update `RedisClient::connect` to call `connect_timeout` with 5s default
7. Write unit tests

## Unit Test Plan

### Test in `tcp.rs` (4 tests):

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `timeout_refused_port` | Connect to localhost:1 (refused, no delay) | Err(Connect), NOT timeout |
| `timeout_unreachable` | Connect to 10.255.255.1:6379 with 1s timeout | Err(Timeout) within 2s |
| `timeout_no_hang` | Same unreachable + 0.5s timeout | Test completes in < 5s (not 75s) |
| `resolve_local_ok` | Connect to 127.0.0.1 with no Redis | Err(Connect), DNS resolves instantly |

### Test in `client.rs` (2 tests):

| Test Name | Setup | Expected |
|-----------|-------|----------|
| `connect_timeout_default_5s` | `connect()` to unreachable host | Uses 5s default |
| `connect_timeout_custom` | `connect_timeout()` with 2s | Uses 2s timeout |

**Note:** The timeout tests require `may::run`/`may::go` setup and a real network namespace. They are integration tests marked `#[ignore]` (require may runtime + network).

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 262 tests pass (timeout tests are `#[ignore]`)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `connect()` still works without timeout argument (uses 5s default)
- [ ] `connect_timeout(host, port, 1)` to unreachable address returns within 2s (not hangs 75s)
- [ ] `connect()` to refused port returns `Err(Connect)` immediately (not `Err(Timeout)`)
- [ ] `ConnectionError::Timeout` variant added and implements `Display` + `Error`
