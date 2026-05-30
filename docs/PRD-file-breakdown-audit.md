# File Breakdown Audit — Updated Post P0-P2 + Follow-up Splits

Last updated: 2026-05-30
Total: 11,291 lines across 59 files (up from 57)
Tests: 319 passed, 0 failed, 36 ignored

## DONE: Priority 0-2 complete

All priority-0 through priority-2 tasks from the original plan are **fully implemented and committed**. No remaining work from that plan.

### Priority 0: Extract test-only files

| Original | New files | Status |
|----------|-----------|--------|
| `protocol/commands_tests.rs` (875 lines) | 8 domain test files under `protocol/commands/` (strings_tests, hashes_tests, sets_tests, lists_tests, sorted_sets_tests, pubsub_tests, transactions_tests, admin_tests) | DONE |
| `client/client_tests.rs` (607 lines) | `client_tests/unit.rs` (120) + `client_tests/integration.rs` (493) | DONE |
| `connection/connection_tests.rs` (489 lines) | Kept as-is (test module, PRD said optional) | DONE |

### Priority 1: Extract embedded tests

| Original | New files | Status |
|----------|-----------|--------|
| `codec/roundtrip.rs` (510 lines) | `roundtrip.rs` (21 prod) + `roundtrip_tests.rs` (495) | DONE |
| `core/from_value.rs` (599 lines) | `from_value.rs` (158 prod) + `from_value_tests.rs` (447) | DONE |
| `codec/reader.rs` (820 lines) | `reader.rs` (340 prod) + `reader_tests.rs` (490) | DONE |
| `client/in_memory.rs` (754 lines) | `in_memory.rs` (346 prod) + `in_memory_tests.rs` (420) | DONE |
| `protocol/builder.rs` (564 lines) | `builder.rs` (291 prod) + `builder_tests.rs` (278) | DONE |

### Priority 2: Split production files

| Original | New files | Status |
|----------|-----------|--------|
| `tls/mod.rs` (552 lines) | `mod.rs` (12) + `config.rs` (149) + `connector.rs` (337) + `tests.rs` (58) | DONE |
| `client/client.rs` (557 lines) | `client.rs` (309) + `client_timeout.rs` (166) + `client_url.rs` (39) + domain impls in client.rs | DONE |
| `connection/tcp.rs` (477 lines) | `tcp.rs` (257) + `tcp_tests.rs` (96) | DONE |
| `connection/connection.rs` (404 lines) | `connection.rs` (217) + `connection_limits.rs` (33) | DONE |
| `connection/connection_io.rs` (368 lines) | Kept as-is (PRD said optional) | DONE |

## FOLLOW-UP SPLITS (completed after PRD)

Three files identified as needing further breakdown were split:

### 3a. `client/pipeline.rs` → `pipeline.rs` + `pipeline_response.rs`

| Original | New files | Status |
|----------|-----------|--------|
| `client/pipeline.rs` (318 lines) | `pipeline.rs` (~155, Pipeline struct + methods) + `pipeline_response.rs` (~160, FromPipelineResponse trait + impls) | DONE |

The Pipeline struct + execute methods (~155) are distinct from FromPipelineResponse trait + 5 impl blocks (~160). Clean separation: pipeline knows nothing about type conversion, pipeline_response knows nothing about command queuing.

### 3b. `tls/connector.rs` → `connector.rs` + `verifier.rs` + `tls_stream.rs`

| Original | New files | Status |
|----------|-----------|--------|
| `tls/connector.rs` (337 lines) | `connector.rs` (~100, TlsConfig + TlsConnector) + `verifier.rs` (~50, SkipVerifier) + `tls_stream.rs` (~50, TlsStream) | DONE |

The SkipVerifier (rustls trait impl), TlsStream (Read/Write impls), and TlsConnector (handshake logic) are fully independent concerns. Config.rs stays as-is (already 149 lines, manageable).

### 3c. `connection/connection_io.rs` → `io_read.rs` + `io_write.rs` + `dispatch.rs` + `epoll_loop.rs`

| Original | New files | Status |
|----------|-----------|--------|
| `connection/connection_io.rs` (368 lines) | `io_read.rs` (~60, nonblock_read) + `io_write.rs` (~40, nonblock_write) + `dispatch.rs` (~120, process_req + decode_responses + error_dispatch) + `epoll_loop.rs` (~170, spawn_connection_loop) | DONE |

The epoll loop is the most complex part (90+ lines of code with 90+ lines of doc comments). Separating I/O helpers from dispatch logic from the loop itself makes each piece independently reviewable. The `process_req` function needed `Request` type, so it lives in dispatch.rs alongside `decode_responses` (both operate on `VecDeque<PendingRequest>`).

## Current State: Production File Inventory

All production files with more than 200 lines:

| File | Lines | Structs | Impl types | Context burden |
|------|-------|---------|------------|----------------|
| `client/in_memory.rs` | 346 | 2 (InMemoryStore, InMemoryClient) | 4 | MEDIUM |
| `codec/reader.rs` | 340 | 1 (RESPReader) | 3 | MEDIUM |
| `tls/connector.rs` | 337 | 4 + 1 enum | 9 impls, 5 types | HIGH |
| `client/pipeline.rs` | 318 | 1 (Pipeline) | 6 | MEDIUM |
| `client/client.rs` | 309 | 2 | 9 | MEDIUM |
| `core/to_args.rs` | 327 | 0 | 10 impls, 10 types | MEDIUM |
| `protocol/builder.rs` | 291 | 2 | 2 | LOW |
| `connection/tcp.rs` | 257 | 2 + 1 enum | 6 impls, 4 types | MEDIUM |
| `codec/writer.rs` | 230 | 1 (RESPWriter) | 2 | LOW |
| `protocol/fake.rs` | 235 | 2 (FakeResponse, FakeConnection) | 2 | LOW |
| `connection/connection.rs` | 217 | 2 (Request, Connection) | 3 | LOW |
| `protocol/commands/admin.rs` | 230 | 0 | 0 (all free fns) | LOW |
| `protocol/commands/strings.rs` | 217 | 0 | 0 (all free fns) | LOW |
| `protocol/commands/sorted_sets.rs` | 216 | 0 | 0 (all free fns) | LOW |

Test files over 250 lines (still impact context during `cargo test --lib`):

| File | Lines |
|------|-------|
| `codec/roundtrip_tests.rs` | 495 |
| `client/client_tests/integration.rs` | 493 |
| `codec/reader_tests.rs` | 490 |
| `connection/connection_tests.rs` | 489 |
| `core/from_value_tests.rs` | 447 |
| `client/in_memory_tests.rs` | 420 |
| `protocol/builder_tests.rs` | 278 |

## Analysis: Which Remaining Files Need Further Breakdown?

Criteria for "needs breakdown":
- **Lines > 200 AND high context burden** (many distinct impl types or large single impl blocks)
- **Contains multiple logical concerns** that could be independently understood
- **Would benefit LLM context windows** by allowing focused reads

### Files that SHOULD be split (high impact on LLM context):

**1. `tls/connector.rs` (337 lines) — HIGH PRIORITY**
- 5 impl types: `TlsError`, `SkipVerifier` (ServerCertVerifier), `TlsConfig`, `TlsStream`, `TlsConnector`
- The `SkipVerifier` (rustls trait impl, ~35 lines), `TlsStream` (Read/Write impls, ~30 lines), and `TlsConnector` (handshake, ~75 lines) are independent concerns
- Split into: `verifier.rs` (~50), `tls_stream.rs` (~50), `connector.rs` (~100), keep config in `config.rs`

**2. `connection/connection_io.rs` (368 lines) — MEDIUM PRIORITY**
- 3 free functions: `process_req` (~15), `nonblock_read` (~30), `nonblock_write` (~20), `decode_responses` (~30), `spawn_connection_loop` (~90)
- The epoll loop in `spawn_connection_loop` has heavy docs (90+ lines of doc comments on just 50 lines of code)
- Split into: `io_read.rs` (~60), `io_write.rs` (~40), `dispatch.rs` (~50), `epoll_loop.rs` (~120)

**3. `client/pipeline.rs` (318 lines) — MEDIUM PRIORITY**
- 120-line `impl Pipeline` block + 6 `FromPipelineResponse` impl blocks for tuples Vec, etc.
- Pipeline struct + methods (~160) is distinct from FromPipelineResponse (~120)
- Split into: `pipeline.rs` (~160), `pipeline_response.rs` (~120)

**4. `client/in_memory.rs` (346 lines) — LOW PRIORITY**
- Two large impl blocks: `InMemoryStore` (132 lines), `InMemoryClient` (122 lines)
- But they're tightly coupled (client wraps store), splitting adds indirection for minimal context gain
- Defer unless code review requires it

**5. `codec/reader.rs` (340 lines) — LOW PRIORITY**
- One massive 254-line `RESPReader` impl block
- Could split by parse strategy (bulk_strings, arrays, integers) but single cohesive type
- The file has good docs per function. Split only if LLM context issues recur.

**6. `client/client.rs` (309 lines) — LOW PRIORITY**
- Already split from 557. Remaining: connect methods (~80), execute (~40), ping+pipeline (~15), domain impls (~15), heavy docs
- The connect methods are already in a separate file conceptually (they're in the struct impl). Splitting further adds negligible context benefit.

### Files that are fine as-is (low context burden):

| File | Why it's OK |
|------|-------------|
| `protocol/builder.rs` (291) | 2 impl types, well-documented, 161-line CommandBuilder is cohesive |
| `core/to_args.rs` (327) | 10 impl types but each is 8-12 lines of simple conversions. Pattern is repetitive and easy to scan. |
| `connection/tcp.rs` (257) | 4 impl types but well-separated: ConnectionError (30), SsrfConfig (105), TcpConnector (120) |
| `codec/writer.rs` (230) | Single RESPONSible struct, 87-line impl is cohesive |
| `protocol/fake.rs` (235) | Test helper, only used in tests, reads quickly |
| `protocol/commands/{admin,strings,sorted_sets}.rs` (217-230) | All free functions, no impl blocks, just command builders |
| `connection/connection.rs` (217) | Two structs with 3 impl blocks, heavy docs but tight cohesion |
| `core/from_value.rs` (158) | Under 200 lines, 10 impl blocks all ~8 lines each, repetitive pattern |

### Test files — status: acceptable

The 6 test files over 400 lines are expected after test extraction (P1). They are:
- `roundtrip_tests.rs` (495) — 45 encode/decode round-trip tests
- `client_tests/integration.rs` (493) — 38 integration tests  
- `reader_tests.rs` (490) — wire decode tests
- `connection_tests.rs` (489) — loop/dispatch/lifecycle tests
- `from_value_tests.rs` (447) — type conversion tests
- `in_memory_tests.rs` (420) — CRUD tests

These are not blocking LLM context issues because:
1. They only load when running `cargo test` (not on `cargo check --lib`)
2. Each test is self-contained and LLMs can reason about individual tests without reading the whole file
3. The pattern is consistent: small assertion blocks, no cross-test dependencies

**Recommendation:** No split needed. If test files become a context issue, split by sub-topic (e.g., roundtrip_tests into `bulk_string_tests.rs`, `array_tests.rs`, etc.).

## Summary: Actionable Recommendations

| # | File | Lines | Action | Effort |
|---|------|-------|--------|--------|
| 1 | `tls/connector.rs` | 337 | Split into 2-3 files (verifier, tls_stream, connector) | Medium |
| 2 | `connection/connection_io.rs` | 368 | Split into io + dispatch submodules | Medium |
| 3 | `client/pipeline.rs` | 318 | Split Pipeline + FromPipelineResponse | Easy |
| 4 | `client/in_memory.rs` | 346 | Defer (tightly coupled) | Low value |
| 5 | `codec/reader.rs` | 340 | Defer (cohesive type) | Low value |

**Bottom line:** Only 3 files have actionable splits with clear benefit. The rest are fine at current size. All production files are under 350 lines. No files exceed 400 lines (all large files are test-only).

## TLS URL Parsing Implementation (Completed 2026-05-30)

Full TLS URL parsing with query parameter support has been implemented:

- `rediss://` URLs are now accepted and routed to TLS connection constructors
- Query parameters parsed: `timeout`, `ca_cert`, `client_cert`, `client_key`, `verify_server`
- Custom CA certs loaded from PEM file paths (comma-separated)
- mTLS support via `client_cert` + `client_key` (PEM parsing)
- `RedisClient::connect_tls()` and `connect_tls_with_ssrf()` constructors added
- `ConnectionStream` enum abstracts over `TcpStream`/`TlsStream` for the connection loop
- `StreamHandle` trait provides unified I/O for the epoll loop
- All 319 tests passing, clippy clean

Example: `rediss://:password@redis.example.com:6380?timeout=10&ca_cert=/path/to/ca.pem&verify_server=true`

## LLM Context Management Assessment

**Before P0-P2:** 8 production files over 350 lines. Worst case: reading one file forced 557 lines of context.

**After P0-P2:** 0 production files over 350 lines. Worst case: 346 lines (`in_memory.rs`).

**Recommendation:** At current state, no immediate action needed. Only 3 files warrant further split (connector.rs, connection_io.rs, pipeline.rs), and all can wait until LLM context issues recur in practice. The codebase is in good shape for LLM-assisted development.
