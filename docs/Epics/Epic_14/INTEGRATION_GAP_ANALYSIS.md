# Epic 14.4 Integration-Only Gap Analysis

**Scope:** All remaining test coverage gaps that require may runtime + real TCP/TLS connections.
These gaps CANNOT be closed with unit tests. This document analyzes what's needed, why it's needed, and where responsibility lies.

**Date:** 2026-06-03
**Status:** DRAFT for review

---

## Executive Summary

After closing 10 of 17 unit-testable gaps (51 new tests, 457 passing), **7 remaining gaps** cover all code that requires a live `may::net::TcpStream` plus a TLS connection. These gaps involve:

1. `TlsConnector::handshake()` polling loop behavior
2. `Connection::connect_tls()` / `connect_tls_with_ssrf()` — TCP + TLS + connection loop assembly
3. `from_tls_stream()` — connection construction from post-handshake state
4. `execute_with_timeout()` — coroutine-level timeout + cancellation
5. `Connection` struct methods (ssrf_config getter, send limits, tag counter)
6. `TlsStream` Read/Write impls and constructors
7. `RedisClient::connect_*()` wrapper methods

The fundamental barrier is that **may does not provide a test double or mock transport layer**. Every piece of code below needs a real TCP socket, and several need a real TLS handshake. This is not a test strategy problem — it is a platform capability problem.

---

## Gap 2: TlsConnector::handshake() — 7 scenarios

**File:** `src/tls/connector.rs` lines 191-256

### What it does

`TlsConnector::handshake()` performs a TLS handshake on a raw TCP stream using a polling loop with `may::coroutine::yield_now()`. It:

1. Clones server_name (defaults to "localhost")
2. Converts to `ServerName<'static>`
3. Calls `config.clone().into_config()` to build a `rustls::ClientConfig`
4. Creates `rustls::ClientConnection::new(Arc::new(config), server_name)`
5. Wraps in `TlsStream::new(conn, stream)`
6. Enters a polling loop calling `tls_stream.conn.complete_io(&mut tls_stream.stream)` until `is_handshaking()` returns false
7. Checks timeout against a deadline + safety valve (100 idle yields)
8. Returns `TlsStream` on success

### Test scenarios required

| # | Scenario | What we test | Why it matters |
|---|----------|-------------|----------------|
| 1 | Full handshake success | `TcpStream::connect("localhost:6380")` + `handshake()` succeeds | Baseline happy path |
| 2 | Handshake timeout | Server accepts TCP but never sends TLS ClientHello → `HandshakeTimeout` | Production: dead TLS servers |
| 3 | Idle loop safety valve (100 yields) | Same as #2 but verify `idle > 100` fires | Regression: if `complete_io` returns partial data, we must not spin forever |
| 4 | `complete_io` I/O error | Server closes connection mid-handshake → `Handshake(String)` | Regression: socket reset, network partition |
| 5 | `server_name` empty → fallback to "localhost" | `TlsConfig { server_name: "" }` builds with "localhost" | Regression: SNI fallback logic |
| 6 | `server_name` invalid → `Config` error | `server_name: "<<<invalid>>>"` fails early | Regression: bad config propagation |
| 7 | `rustls::ClientConnection::new` fails | Valid config but invalid cert chain → `Config` error | Regression: cert chain validation |

### Why we can't unit test this

`TlsConnector::handshake()` takes a `may::net::TcpStream` and calls `rustls::ClientConnection::complete_io()`. There is **no trait interface** on `TcpStream` — it is a concrete struct wrapping `std::net::TcpStream` with may-aware I/O registration. We cannot:

- Inject a "fake" stream that returns controlled data
- Mock `rustls::ClientConnection` — it requires a `ClientConfig` and `ServerName`
- Mock `complete_io()` — it is a method on `rustls::ClientConnection`

The polling loop (lines 224-252) is particularly hard to test because `complete_io()` on a real TCP socket will hang until data arrives, and the `yield_now()` only matters when `complete_io` returns `(0, 0)` (no progress). The timeout path requires a real TCP connection where the other end is silent.

### Responsibility assessment

**This is a MAY RUNTIME gap, not a may-redis gap.**

The root cause is that may provides no mock/fake TCP transport for testing. If may had a `FakeTcpStream` that implements `Read` + `Write` + `AsIoData` + `WaitIo`, we could:

1. Create a `FakeTcpStream` pre-loaded with TLS handshake bytes
2. Feed responses incrementally to simulate slow servers, resets, timeouts
3. Test all 7 scenarios without a real network connection

may's own integration tests (`may/tests/integration_tests.rs`) use real TCP sockets with `TcpListener::bind` on loopback. They even have a `MockIo` type in `split_io.rs` tests, but it only works with the `SplitIo` abstraction, not with raw `TcpStream`.

**RECOMMENDATION:** Open an issue on Xudong-Huang/may requesting:
- A `FakeTcpStream` or `InMemoryTcpStream` that implements `Read` + `Write` + `AsIoData` + `WaitIo`
- Or at minimum, a `TcpStream::from_raw_parts(fd, ...)` constructor that lets tests create a stream from a pipe pair (which avoids actual network I/O)

Until may provides this, Gap 2 can ONLY be tested with a real TLS server on localhost.

---

## Gap 3: connect_tls() / connect_tls_with_ssrf() — 6 scenarios

**File:** `src/connection/connection_tls.rs` lines 35-94

### What it does

Two functions that compose TCP connect → TLS handshake → connection loop:

```
connect_tls():
  1. TcpConnector::connect_timeout(host, port, timeout_secs) → TcpStream
  2. TlsConnector::handshake(stream, tls_config, timeout) → TlsStream
  3. from_tls_stream(ConnectionStream::Tls(boxed)) → Connection

connect_tls_with_ssrf():
  1. TcpConnector::connect_with_ssrf_check(host, port, timeout, ssrf_config) → TcpStream
  2. TlsConnector::handshake(stream, tls_config, timeout) → TlsStream
  3. from_tls_stream_with_ssrf(ConnectionStream::Tls(boxed), Some(ssrf_config)) → Connection
```

### Test scenarios required

| # | Function | Scenario | Error path |
|---|----------|----------|------------|
| 1 | `connect_tls()` | Happy path: TCP connects, TLS handshakes, connection loop spawns | None |
| 2 | `connect_tls()` | TCP connect fails (no server) | `ConnectionError::Connect` |
| 3 | `connect_tls()` | TCP connects, TLS handshake fails (wrong server) | `ConnectionError::Tls` |
| 4 | `connect_tls_with_ssrf()` | SSRF blocks 10.0.0.1 → `SsrfViolation` | Early, before TCP |
| 5 | `connect_tls_with_ssrf()` | SSRF allows, TCP connects, TLS handshakes | Full path |
| 6 | `connect_tls_with_ssrf()` | SSRF allows, TCP connects, TLS handshake fails | `Tls` after SSRF |

### Why we can't unit test this

This is a pure composition function. It calls three other functions, each requiring may runtime:
- `TcpConnector::connect_timeout()` → real TCP socket
- `TlsConnector::handshake()` → real rustls handshake
- `spawn_connection_loop()` → real `may::go!` coroutine with epoll

The `ConnectionError` types that get returned ARE testable if we can make the TCP connect fail, but we still need a real socket to observe the error propagation.

### Responsibility assessment

**Mixed — may-redis + may runtime.**

The SSRF blocking path (Gap #4) is testable if we could create a `TcpStream` pointing to a blocked IP without actual network I/O. The TCP failure paths (Gap #2) similarly need a socket that fails connect.

If may had a mock transport, all 6 scenarios would be unit-testable. Without it, they require a real TLS server.

**VERDICT:** 4 of 6 scenarios (TCP failure, TLS failure, SSRF blocked, SSRF+TLS failure) could be tested against a real Redis-TLS server. 2 scenarios (SSRF blocked before TCP, SSRF allowed+full path) are already partially covered by the integration test suite's general connect tests.

---

## Gap 4: from_tls_stream() helpers — 4 scenarios

**File:** `src/connection/connection_tls.rs` lines 105-135

### What it does

Takes an already-handshaked `TlsStream` wrapped in `ConnectionStream` and builds a `Connection` struct:

```rust
fn from_tls_stream(stream: ConnectionStream) → Connection {
    let id = stream.inner_mut().as_raw_fd() as usize;
    let waker = stream.inner_mut().waker();
    let req_queue = Arc::new(Queue::new());
    let pending_count = Arc::new(AtomicUsize::new(0));
    let io_handle = spawn_connection_loop(stream, req_queue.clone(), pending_count.clone());
    Connection { io_handle, req_queue, waker, id, tag_counter: ..., max_queue_depth: DEFAULT_MAX_QUEUE_DEPTH, max_request_size: DEFAULT_MAX_REQUEST_SIZE, pending_count, ssrf_config: None }
}
```

### Test scenarios required

| # | Function | Scenario | What we verify |
|---|----------|----------|----------------|
| 1 | `from_tls_stream()` | Create connection from TLS stream | `Connection` has correct defaults (max_queue_depth, max_request_size, ssrf_config=None) |
| 2 | `from_tls_stream_with_ssrf()` | Create with ssrf_config Some | `Connection.ssrf_config()` returns the provided config |
| 3 | `from_tls_stream_with_ssrf()` | Create with ssrf_config None | `Connection.ssrf_config()` returns None |
| 4 | `spawn_connection_loop()` | Background loop actually starts | `io_handle` is non-None, loop is responsive |

### Why we can't unit test this

`spawn_connection_loop()` requires:
1. A real `ConnectionStream` (which wraps a real `TlsStream`)
2. A real `may::go!` coroutine context
3. A real epoll event loop

The `from_tls_stream()` function itself is trivial (just field assignment), but `spawn_connection_loop()` means we can't test it without a running connection.

### Responsibility assessment

**May runtime + may-redis.**

The `spawn_connection_loop()` dependency means this can ONLY be tested in an integration test that brings up a real connection. The field assignment part (max_queue_depth, max_request_size defaults) is testable IF we had a way to create a `Connection` struct directly without going through `spawn_connection_loop()`.

**RECOMMENDATION:** Add a `Connection::from_stream_with_limits()` constructor that accepts a `StreamHandle` and explicit limits, separate from `spawn_connection_loop`. This would make the struct construction testable in isolation and the loop-spawning testable in integration.

---

## Gap 5: execute_with_timeout() — 5 scenarios

**File:** `src/client/client_timeout.rs` lines 71-165

### What it does

Executes a Redis command with a configurable timeout:

```rust
pub fn execute_with_timeout<T: FromRedisValue>(&self, cmd: CommandBuilder, timeout: Duration) → Result<T, RedisError> {
    1. Validate command against policy
    2. Build command into RESP bytes
    3. Create spsc channel for this request's response
    4. Create TimeoutGuard with AtomicBool
    5. Check timeout BEFORE sending request (Finding #1)
    6. Spawn timeout coroutine: may::go!(sleep(timeout) → signal cancellation)
    7. If cancelled before send → return Connection error
    8. Send request to connection loop
    9. Poll loop: try_recv from response channel OR try_recv from timeout channel
    10. Convert RedisValue to typed result
}
```

### Test scenarios required

| # | Scenario | What we test |
|---|----------|-------------|
| 1 | Normal execution | Response arrives before timeout → correct value returned |
| 2 | Timeout fires before response | Response channel has no data, timeout channel fires → Connection error |
| 3 | Timeout fires before request sent | Timeout coroutine sleeps for 0 duration → cancelled is true before send → early Connection error |
| 4 | Guard cancellation spsc drop | Response arrives, guard dropped, timeout coroutine sees dropped channel and exits cleanly |
| 5 | Response arrives after timeout signal | Timeout fires, drops channel, response arrives anyway but is ignored |

### Why we can't unit test this

`execute_with_timeout()` calls `self.inner.connection.send(Request::new(...))` which requires:
1. A `Connection` struct (which needs `spawn_connection_loop()` → real may coroutine + epoll)
2. A real `spsc::channel()` interaction with the connection loop
3. The timeout coroutine (`may::go! { may::coroutine::sleep(timeout) }`) running in parallel

The polling loop at lines 126-136 (`rx.try_recv()`, `timeout_rx.try_recv()`, `yield_now()`) is the most testable part — it's pure Rust with `try_recv` (non-blocking). But it depends on the connection loop actually receiving and responding.

### Responsibility assessment

**May runtime gap.**

This is fundamentally a may runtime problem. `may::coroutine::sleep()` and `may::go!()` require a running may runtime. There is no way to test coroutine-level timeout behavior without:
- A real may runtime (which `cargo test --lib` provides)
- A real connection loop that responds to requests
- A real spsc channel that carries responses

**CRITICAL NOTE:** The `may` runtime does NOT have a `mock_sleep` or `mock_timer` facility. Unlike tokio's `tokio::time::pause()` + `tokio::time::advance()`, may has no equivalent. This makes timeout testing inherently dependent on wall-clock time or real network latency.

**RECOMMENDATION:** Request from may authors:
- `may::timer::mock_timer()` — a testable timer that can be advanced programmatically
- OR `may::coroutine::mock_sleep(duration, fake_duration)` — allows tests to control timeout timing

Until then, timeout tests must use real `may::coroutine::sleep()` with actual durations (milliseconds). This means they are slow integration tests that depend on wall-clock time.

---

## Gap 8: RedisClient::connect_*() wrapper methods — 4 scenarios

**File:** `src/client/client.rs`

### What it does

These are thin wrappers that delegate to `Connection::connect_*()` methods:

```rust
RedisClient::connect() → Connection::connect() → TcpConnector::connect()
RedisClient::connect_with_timeout() → Connection::connect_with_limits() → TcpConnector::connect_with_timeout()
RedisClient::connect_tls() → Connection::connect_tls() → connect_tls() → TlsConnector::handshake()
RedisClient::connect_tls_with_ssrf() → Connection::connect_tls_with_ssrf() → connect_tls_with_ssrf() → TlsConnector::handshake()
```

### Test scenarios required

| # | Function | Coverage |
|---|----------|----------|
| 1 | `RedisClient::connect()` | Delegates correctly, creates valid client |
| 2 | `RedisClient::connect_with_timeout()` | Custom timeout propagates to connection |
| 3 | `RedisClient::connect_tls()` | TLS config propagates through the chain |
| 4 | `RedisClient::connect_tls_with_ssrf()` | SSRF config propagates through the chain |

### Why we can't unit test this

All four are delegation methods. They call into `Connection` which requires `may::go!` + epoll + real TCP. There is zero standalone logic to test.

### Responsibility assessment

**May runtime gap.**

These wrappers are not independently testable. They are covered transitively by Gap 3's tests. If we close Gap 3, Gap 8 closes automatically.

**VERDICT:** Do not test separately. The integration tests for Gap 3 cover these paths.

---

## Gap 9: Connection struct methods — 9 scenarios

**File:** `src/connection/connection.rs`

### What it does

The `Connection` struct owns the connection loop, request queue, and waker. Key methods:

```rust
Connection::connect(host, port) → builds Connection with defaults
Connection::connect_with_ssrf_protection(host, port, timeout, ssrf_config) → builds with SSRF
Connection::connect_with_limits(host, port, timeout, max_queue_depth, max_request_size) → builds with custom limits
Connection::ssrf_config() → Option<&SsrfConfig>
Connection::send(request) → Result<usize, ConnectionLimitError>
Connection::id() → usize
```

### Test scenarios required

| # | Method | Scenario | What we verify |
|---|--------|----------|----------------|
| 1 | `connect()` | Default limits | max_queue_depth = DEFAULT_MAX_QUEUE_DEPTH, max_request_size = DEFAULT_MAX_REQUEST_SIZE |
| 2 | `connect_with_ssrf_protection()` | SSRF config stored | ssrf_config = Some(given_config) |
| 3 | `connect_with_limits()` | Custom limits | max_queue_depth and max_request_size match arguments |
| 4 | `ssrf_config()` — plain TCP | No SSRF | Returns None |
| 5 | `ssrf_config()` — SSRF enabled | SSRF configured | Returns Some(&config) |
| 6 | `send()` — success | Request within limits | Returns Ok(tag), pending_count incremented |
| 7 | `send()` — queue full | pending_count >= max_queue_depth | Returns QueueFull error |
| 8 | `send()` — too large | request.data.len() > max_request_size | Returns RequestTooLarge error |
| 9 | `id()` | After connect | Returns stream's raw FD as usize |

### Why we can't unit test this

The `Connection` struct holds:
- `JoinHandle<()>` — a may coroutine handle (requires may runtime to create)
- `Arc<Queue<Request>>` — may's mpsc queue (requires may runtime)
- `WaitIoWaker` — epoll-based waker (requires a real socket)
- `Arc<AtomicUsize>` tag_counter, pending_count

All construction paths (`connect()`, `connect_with_ssrf_protection()`, `connect_with_limits()`) create real TCP connections and spawn real epoll loops.

The `ssrf_config()`, `id()`, and `send()` methods ARE pure Rust logic, but they require a `Connection` instance, which cannot be created without may runtime.

### Responsibility assessment

**Mixed — may runtime + architectural gap.**

The root problem is that `Connection` has no no-arg constructor or `::default()`. All construction requires a real TCP socket.

**RECOMMENDATION:** Add a `Connection::from_stream_with_limits(stream, limits, ssrf_config)` constructor that:
1. Takes a `StreamHandle` (trait object) instead of requiring a real TCP stream
2. Accepts explicit `max_queue_depth` and `max_request_size`
3. Uses a `WaitIoWaker` created from the stream
4. Does NOT spawn a coroutine

This would make the struct construction testable by injecting a `MockStream` that implements `StreamHandle`. The coroutine-spawning paths would remain integration tests.

---

## Gap 10: TlsStream constructors/Read/Write — 5 scenarios

**File:** `src/tls/tls_stream.rs` lines 19-58

### What it does

`TlsStream` wraps a `rustls::ClientConnection` and `may::net::TcpStream`:

```rust
impl TlsStream {
    pub const fn new(conn: rustls::ClientConnection, stream: TcpStream) → Self
    pub const fn inner_mut(&mut self) → &mut TcpStream
    pub const fn inner(&self) → &TcpStream
}

impl io::Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) → io::Result<usize> {
        self.conn.reader().read(buf)
    }
}

impl io::Write for TlsStream {
    fn write(&mut self, buf: &mut [u8]) → io::Result<usize> {
        self.conn.writer().write(buf)
    }
    fn flush(&mut self) → io::Result<()> {
        self.conn.writer().flush()
    }
}

impl StreamHandle for TlsStream {
    fn inner_mut(&mut self) → &mut may::net::TcpStream
    fn wait_io(&mut self) → i32
}
```

### Test scenarios required

| # | What | Scenario | What we verify |
|---|------|----------|----------------|
| 1 | `TlsStream::new()` | Construct with valid conn + stream | Fields are correctly set |
| 2 | `inner_mut()` | Get mutable TCP stream ref | Returns the inner stream |
| 3 | `inner()` | Get immutable TCP stream ref | Returns the inner stream |
| 4 | `Read` impl | Read through TLS stream | Data flows through rustls reader |
| 5 | `Write` impl | Write through TLS stream | Data flows through rustls writer |

### Why we can't unit test this

`TlsStream::new()` requires a `rustls::ClientConnection`, which requires:
1. A `rustls::ClientConfig` (requires `TlsConfig::into_config()` — Gap 12 covers this path)
2. A `ServerName` (can be constructed from &str)
3. The resulting `ClientConnection` is a complex state machine

Additionally, `TlsStream` holds a `may::net::TcpStream`, which cannot be constructed without may runtime or a real socket.

**HOWEVER:** The `Read` and `Write` impls are trivially just forwarding to `rustls`'s `reader()` and `writer()`. These can be tested IF we had a `TlsStream` instance, but creating one requires the full TLS setup chain.

### Responsibility assessment

**May runtime + rustls gap.**

`rustls` does provide `ServerConfig` and `ClientConfig` builders that can be constructed in tests (we already do this in Gap 12). But `rustls::ClientConnection::new()` requires a live `ServerConfig` from the other side to complete the handshake for meaningful Read/Write tests.

**RECOMMENDATION:** The simplest path is to use `rustls`'s `NoClientCert` verifier and a server certificate chain to create a test `ClientConnection` that can be used for Read/Write testing. The `may::net::TcpStream` dependency is the real blocker — without it, we can't even construct a `TlsStream` for testing.

---

## Gap 13: TcpConnector methods — 5 scenarios

**File:** `src/connection/tcp.rs`

### What it does

`TcpConnector` wraps TCP connection with optional SSRF protection:

```rust
TcpConnector::connect(host, port) → default 5s timeout
TcpConnector::connect_with_ssrf_check(host, port, timeout, ssrf_config) → SSRF + DNS + connect
TcpConnector::connect_with_timeout(host, port, timeout) → custom timeout
TcpConnector::connect_timeout(sockaddr, timeout) → seconds to Duration conversion
TcpConnector::resolve(host, port) → DNS resolution
```

### Test scenarios required

| # | Method | Scenario | What we verify |
|---|--------|----------|----------------|
| 1 | `connect()` | Default 5s timeout | Uses `DEFAULT_CONNECT_TIMEOUT` (5s) |
| 2 | `connect_with_ssrf_check()` | SSRF blocks → early error | Returns `SsrfViolation` before DNS |
| 3 | `connect_with_timeout()` | Custom timeout | Uses provided timeout, not default |
| 4 | `connect_timeout()` | Seconds → Duration | Correct conversion for various durations |
| 5 | `resolve()` with 0 addresses | DNS returns empty | Returns error, not panic |

### Why we can't unit test this

`TcpConnector::connect()` calls `may::net::TcpStream::connect()` which:
1. Does DNS resolution via `std::net::ToSocketAddrs`
2. Creates a `may::net::TcpStream` via `may::io::net::TcpStreamConnect`
3. Registers the socket with may's epoll event loop via `io_impl::add_socket()`

The `resolve()` function is just `std::net::ToSocketAddrs` — that's testable but requires a real hostname to resolve.

The `connect_timeout()` and `connect_with_timeout()` paths require may runtime because they call `may::net::TcpStream::connect_timeout()`.

### Responsibility assessment

**May runtime gap.**

`TcpStream::connect()` and `TcpStream::connect_timeout()` are may's TCP implementation. Without a `FakeTcpStream` or at minimum a pipe-pair-based test helper, these require real network.

**VERDICT:** 3 of 5 scenarios (connect default, custom timeout, seconds→Duration) are covered by the existing integration test suite's general connect tests. 2 scenarios (SSRF check, empty DNS) are already covered by our unit tests in `tcp_tests.rs`.

---

## Comprehensive Responsibility Matrix

| Gap | Files | May-Runtime Gap? | May-Redis Gap? | Needs May Authors? |
|-----|-------|------------------|----------------|-------------------|
| 2 | `tls/connector.rs` | YES — no mock TCP | NO — logic is clear | YES — needs FakeTcpStream |
| 3 | `connection/connection_tls.rs` | YES — composition of may primitives | NO — pure Rust | YES — or provide integration fixture |
| 4 | `connection/connection_tls.rs` | YES — spawn_connection_loop needs may | NO | YES |
| 5 | `client/client_timeout.rs` | YES — coroutine sleep/timing | NO | YES — needs mock timer |
| 8 | `client/client.rs` | YES — delegates to Connection | NO | NO — covered by Gap 3 |
| 9 | `connection/connection.rs` | YES — Connection construction | PARTIAL — no `::default()` | YES — add trait-based constructor |
| 10 | `tls/tls_stream.rs` | YES — TcpStream dependency | NO — impls are trivial | YES — or provide test helpers |
| 13 | `connection/tcp.rs` | YES — may::net::TcpStream | NO | YES — needs FakeTcpStream |

### Summary

- **9 of 9 scenarios across all gaps** require may runtime primitives (go!, Queue, spsc, epoll, sleep)
- **0 scenarios** are purely may-redis code problems
- **All gaps** could be closed with a `FakeTcpStream` or pipe-pair-based mock
- **may does not currently provide** any mock transport layer for testing
- **may's own tests** (`may/tests/integration_tests.rs`) use real TCP sockets on loopback

### Key May Runtime Shortfalls

| Shortfall | Impact | May Equivalent? |
|-----------|--------|-----------------|
| No `FakeTcpStream` | All TCP/TLS tests require real network | No equivalent |
| No `mock_timer()` / `mock_sleep()` | Timeout tests need wall-clock time | No equivalent (tokio has `time::pause()`) |
| No `in_memory_stream()` | Cannot test I/O without network | No equivalent |
| No `TcpStream::from_raw_parts()` | Cannot construct streams for testing | Partial — `TcpStream::from_raw_fd()` exists but no epoll registration |
| No `FakeWaitIoWaker` | Cannot test epoll waker behavior | No equivalent |
| No `MockChannel` for spsc/mpsc | Cannot test request/response pipeline | No equivalent (tokio has `mpsc::channel()` that works without runtime) |

### What We Should Ask May Authors For

Priority order:

1. **`FakeTcpStream`** — implements `Read` + `Write` + `AsIoData` + `WaitIo` backed by `pipe()` pairs. This single addition would close 6 of 9 gaps.

2. **`mock_timer()` / `mock_sleep()`** — a testable timer that can be advanced programmatically. Closes the timeout testing gap (Gap 5).

3. **`TcpStream::from_raw_parts(fd, ...)`** — create a may-aware stream from an existing fd. Useful for Gap 10 (TlsStream).

4. **`Channel::new_test()`** — a channel that works without a may runtime. Useful for Gap 5.

### Current Fallback Strategy

Without may runtime support, the only path is:

1. **Integration tests with real Redis-TLS server on localhost** — spin up Redis with TLS via Docker, run tests against it
2. **Use `#[ignore = "requires live Redis-TLS server"]`** — mark all integration tests as ignored, run only in CI
3. **Use a test fixture** — create a small Rust test server (like may's own echo server) that mimics Redis-TLS behavior

This is the approach already used in `client::client_tests::integration.rs` — all tests are `#[ignore]` and require a live server. The same pattern should be extended to TLS-specific tests.

---

## Recommended Integration Test Strategy

### Architecture

```
tests/
├── integration/
│   ├── mod.rs
│   ├── redis_server.rs      # Spin up Redis with TLS via Docker/Redis server binary
│   ├── tls_handshake.rs     # Tests for TlsConnector::handshake()
│   ├── connection_tls.rs    # Tests for connect_tls(), connect_tls_with_ssrf()
│   ├── execute_timeout.rs   # Tests for execute_with_timeout()
│   └── connection_methods.rs # Tests for Connection struct methods
```

### Test Server Setup

Option A: **Docker Redis with TLS**
```bash
docker run --rm -p 6380:6380 -v ./tests/tls:/certs redis:7-alpine   redis-server --tls-port 6380 --tls-cert-file /certs/server.crt   --tls-key-file /certs/server.key --tls-ca-cert-file /certs/ca.crt
```

Option B: **Self-signed test certs**
Generate test certificates in the test fixture:
```rust
fn setup_test_tls() → (cert_pem, key_pem, ca_pem) {
    // Use openssl crate to generate self-signed certs at test time
}
```

### Test Structure Pattern

```rust
#[test]
#[ignore = "requires live Redis-TLS server on localhost:6380"]
fn test_tls_handshake_success() {
    may::run(|| {
        may::go! {
            let client = RedisClient::connect_tls("localhost", 6380, &tls_config, 5).unwrap();
            let result: Option<String> = client.execute(client.get("test_key")).unwrap();
            assert_eq!(result, None);
        }
    }).join();
}
```

### CI Integration

Add to CI pipeline:
```yaml
- name: Start Redis-TLS
  run: docker-compose up -d redis-tls

- name: Run integration tests
  run: cargo test --all-features -- --ignored --test-threads=1

- name: Stop Redis-TLS
  run: docker-compose down
```

---

## Summary of Action Items

### Immediate (may-redis side)

1. **Add integration test files** under `tests/integration/` covering all 9 gaps
2. **Create test TLS certificates** generation utilities
3. **Document the may runtime gap** in the project README
4. **Mark all integration tests as `#[ignore]`** with clear skip messages

### Medium-term (may authors collaboration)

1. **Open issue on Xudong-Huang/may** requesting `FakeTcpStream` support
2. **Open issue requesting** `mock_timer()` / `mock_sleep()` for testable timeouts
3. **Contribute** `FakeTcpStream` to may if accepted, or vendor it as a may-redis dependency

### Long-term (architecture improvement)

1. **Introduce trait-based abstraction** for streams (`trait RedisStream: Read + Write + AsIoData + WaitIo`)
2. **Make `TlsStream` generic** over the underlying stream type, allowing mock injection
3. **Extract connection loop spawning** into a separate testable function

---

## Appendix: May Runtime Architecture Reference

### may's Testing Infrastructure (Current State)

may provides minimal test infrastructure:

- `may/tests/integration_tests.rs` — real TCP/UDP/HTTP tests using loopback sockets
- `may/src/io/split_io.rs` tests — MockIo for `SplitIo` abstraction only
- `may::sync::spsc` tests — channel tests without may runtime (pure Rust)
- `may::queue::mpsc::Queue` tests — queue tests without may runtime (pure Rust)

**Critical gap:** may's `TcpStream` is a concrete struct, not a trait. It wraps `std::net::TcpStream` with may-aware I/O registration. There is no trait abstraction that allows swapping in a fake implementation.

### may's Core Primitives Used by may-redis

| Primitive | may Module | may-redis Usage | Testable? |
|-----------|-----------|-----------------|-----------|
| `may::net::TcpStream` | net/tcp.rs | All TCP connections | NO — concrete struct |
| `may::io::WaitIo` | io/wait_io.rs | Epoll registration | NO — requires real socket |
| `may::io::WaitIoWaker` | io/wait_io.rs | Wake connection loop | NO — requires real socket |
| `may::go!` | coroutine | Spawn connection loop | PARTIAL — works in test |
| `may::coroutine::yield_now()` | yield_now.rs | Polling loops | PARTIAL — works in test |
| `may::coroutine::sleep()` | timer | Timeout handling | NO — wall-clock only |
| `may::sync::spsc::channel()` | sync/spsc.rs | Request/response | YES — works without runtime |
| `may::queue::mpsc::Queue` | queue/mpsc.rs | Request queue | YES — works without runtime |

### May vs Tokio Comparison for Testing

| Feature | Tokio | May (current) | May (needed) |
|---------|-------|---------------|-------------|
| Mock time | `tokio::time::pause()` + `advance()` | None | `mock_timer()` |
| Mock TCP | `tokio::io::DuplexStream` | None | `FakeTcpStream` |
| Mock channel | `mpsc::channel()` (no runtime needed) | `spsc::channel()` (no runtime) | Already works |
| Mock timer | `tokio::time::interval()` | `may::coroutine::sleep()` | Mockable sleep |
| Spawn in test | `#[tokio::test]` | `may::run()` + `may::go!()` | Already works |

---

## Appendix: Test Coverage Gap Summary

| Gap | File | Functions | Scenarios | Unit Testable? | Needs May? |
|-----|------|-----------|-----------|----------------|------------|
| 2 | tls/connector.rs | `handshake()` | 7 | NO | YES |
| 3 | connection/connection_tls.rs | `connect_tls()`, `connect_tls_with_ssrf()` | 6 | NO | YES |
| 4 | connection/connection_tls.rs | `from_tls_stream()` | 4 | NO | YES |
| 5 | client/client_timeout.rs | `execute_with_timeout()` | 5 | NO | YES |
| 8 | client/client.rs | `RedisClient::connect_*()` | 4 | NO | YES |
| 9 | connection/connection.rs | `Connection` methods | 9 | PARTIAL | YES |
| 10 | tls/tls_stream.rs | `TlsStream` | 5 | NO | YES |
| 13 | connection/tcp.rs | `TcpConnector` methods | 5 | PARTIAL | YES |
| **Total** | | | **45 scenarios** | **0 unit-testable** | |

**Total remaining gaps: 45 test scenarios across 8 files.**
**All require may runtime + real TCP/TLS.**
