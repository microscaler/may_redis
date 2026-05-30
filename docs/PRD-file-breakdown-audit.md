# PRD: File Size Audit & Breakdown Plan

## Goal
Identify how to break down files exceeding 350 lines into smaller, focused modules following the modular architecture outlined in Story 0 of each epic.

## Current State (post-connection split)

### Files OVER 350 lines: 14

| # | File | Lines | Status |
|---|------|-------|--------|
| 1 | `client/client.rs` | 1152 | OVER — needs split |
| 2 | `protocol/commands_tests.rs` | 876 | OVER — test extraction needed |
| 3 | `codec/reader.rs` | 820 | OVER — needs split |
| 4 | `client/in_memory.rs` | 754 | OVER — needs split |
| 5 | `core/from_value.rs` | 599 | OVER — needs split |
| 6 | `protocol/builder.rs` | 564 | OVER — needs split |
| 7 | `tls/mod.rs` | 552 | OVER — needs split |
| 8 | `codec/roundtrip.rs` | 510 | OVER — needs split |
| 9 | `connection/connection_tests.rs` | 489 | OVER — test extraction |
| 10 | `connection/tcp.rs` | 477 | OVER — needs split |
| 11 | `connection/connection.rs` | 404 | DONE — already split |
| 12 | `connection/connection_io.rs` | 371 | DONE — already split |
| 13 | `core/error.rs` | 360 | NEAR — extract tests |
| 14 | `protocol/fake.rs` | 354 | OK — just under |

### Files under 350 lines: 20 (OK)

## Phase 1: Trivial wins (extract tests only)

### Priority 1.1: `core/error.rs` (360 lines)
**Action:** Extract `#[cfg(test)]` module to `error_tests.rs`

**Analysis:**
- Line 141: `#[cfg(test)]` starts
- ~18 lines of tests
- Result: ~342 lines

**Steps:**
1. Extract `#[cfg(test)]` block from end of `core/error.rs` to `core/error_tests.rs`
2. Strip 4-space indentation from test content
3. Add explicit imports to `error_tests.rs`
4. Wire up `mod.rs`: add `#[cfg(test)] mod error_tests;`

---

### Priority 1.2: `protocol/fake.rs` (354 lines)
**Action:** Extract `#[cfg(test)]` module to `fake_tests.rs`

**Analysis:**
- Line 236: `#[cfg(test)]` starts
- ~12 lines of tests
- Result: ~342 lines

**Steps:** Same as 1.1 pattern.

---

### Priority 1.3: `connection/connection_tests.rs` (489 lines)
**Action:** This is the test module for `connection.rs` — already extracted. No further action needed here; this is by design.

---

## Phase 2: Module extraction (extract I/O, helpers, tests)

### Priority 2.1: `codec/reader.rs` (820 lines)

**Structure analysis needed:** This is the largest remaining file. I need to identify logical groupings.

**Hypothesis:** Contains:
- `RESPReader` struct definition and core impl
- Helper methods for RESP parsing
- Test module

**Proposed split:**
- `reader.rs` — `RESPReader` struct, constructor, public API
- `reader_impl.rs` — private helper methods
- `reader_tests.rs` — extracted tests

**Estimated result:** ~250-300 lines per file

---

### Priority 2.2: `codec/roundtrip.rs` (510 lines)

**Structure analysis needed:** Likely a mix of encode/decode round-trip tests.

**Hypothesis:** Contains test functions for each RESP type.

**Proposed split:**
- `roundtrip.rs` — test fixture/harness
- `roundtrip_tests.rs` — extracted test cases (likely >400 lines)

**Estimated result:** ~100 lines + ~400 lines

---

### Priority 2.3: `tls/mod.rs` (552 lines)

**Structure analysis needed:** TLS configuration and connection setup.

**Hypothesis:** Contains:
- TLS config structs
- Connector logic
- Test module

**Proposed split:**
- `tls_config.rs` — config types
- `tls_connector.rs` — connection logic
- `tls_tests.rs` — extracted tests

**Estimated result:** ~200 lines each

---

## Phase 3: Client layer split

### Priority 3.1: `client/client.rs` (1152 lines)

**This is the single largest file and highest priority.**

**Structure analysis needed:** Contains `RedisClient` struct and Commands trait implementations.

**Proposed split (following the modular architecture):**
- `client.rs` — `RedisClient` struct, connection management, lifecycle
- `client_impl.rs` — `impl Connection` helpers
- `client_commands.rs` — Commands trait method implementations (~122 methods)
- `client_tests.rs` — extracted tests

**Reference:** The Commands are already split into 8 domain modules (`protocol/commands/`). The client should similarly delegate to these.

**Estimated result:** ~250 lines each for 4 files

---

## Phase 4: Protocol layer split

### Priority 4.1: `protocol/builder.rs` (564 lines)

**Hypothesis:** Contains `CommandBuilder` and `Commands` trait implementation.

**Proposed split:**
- `builder.rs` — `CommandBuilder` struct, constructor, builder methods
- `builder_impl.rs` — private helpers
- `builder_tests.rs` — extracted tests

**Estimated result:** ~200-250 lines each

---

### Priority 4.2: `core/from_value.rs` (599 lines)

**Hypothesis:** Contains `FromRedisValue` derive/macro implementations and parsing logic.

**Proposed split:**
- `from_value.rs` — `FromRedisValue` trait definition
- `from_value_impl.rs` — implementations per type
- `from_value_tests.rs` — extracted tests

**Estimated result:** ~200 lines each

---

## Phase 5: Client utilities

### Priority 5.1: `client/in_memory.rs` (754 lines)

**Hypothesis:** In-memory Redis implementation for testing.

**Proposed split:**
- `in_memory.rs` — `InMemoryClient` struct
- `in_memory_impl.rs` — Commands implementation for in-memory backend
- `in_memory_tests.rs` — extracted tests

**Estimated result:** ~250 lines each

---

### Priority 5.2: `connection/tcp.rs` (477 lines)

**Hypothesis:** TCP connector with SSRF protection.

**Proposed split:**
- `tcp.rs` — `TcpConnector` struct, connection helpers
- `tcp/ssrf.rs` — SSRF configuration and IP checking
- `tcp_tests.rs` — extracted tests

**Estimated result:** ~200 lines each

---

## Verification Steps (after each phase)

1. `cargo check --lib` — must compile
2. `cargo test --lib` — all tests must pass
3. Verify file line count is under 350
4. Verify no new clippy warnings

## Target State

After all phases complete:
- Maximum file size: <350 lines
- Zero 1000+ line files (down from 1)
- Zero 800+ line files (down from 1)
- Total files: 34 + ~20 extracted = ~54 files
- All files under 350 lines

## Progress

### Completed

- [x] Phase 1: Trivial wins (extract tests)
  - [x] 1.1: `core/error.rs` (360 → 138 lines; `error_tests.rs`: 220 lines)
  - [x] 1.2: `protocol/fake.rs` (354 → 235 lines; `fake_tests.rs`: 119 lines)
- [x] Connection module pre-split (done in prior session):
  - [x] `connection.rs` (1238 → 404 lines) — I/O helpers removed, struct/impl kept
  - [x] `connection_io.rs` (371 lines) — I/O loop, process_req, decode_responses
  - [x] `connection_tests.rs` (489 lines) — extracted test module

### Current Status

**Files OVER 350 lines: 10** (down from 14 → 12 → 10)
**Files under/equal 350 lines: 26** (up from 20 → 24 → 26)
**Total files: 38** (was 34; +4 new files created)

### Remaining work

- [ ] Phase 2: Module extraction
  - [ ] 2.1: `codec/reader.rs` (820 lines)
  - [ ] 2.2: `codec/roundtrip.rs` (510 lines)
  - [ ] 2.3: `tls/mod.rs` (552 lines)
- [ ] Phase 3: Client layer
  - [ ] 3.1: `client/client.rs` (1152 lines)
- [ ] Phase 4: Protocol layer
  - [ ] 4.1: `protocol/builder.rs` (564 lines)
  - [ ] 4.2: `core/from_value.rs` (599 lines)
- [ ] Phase 5: Client utilities
  - [ ] 5.1: `client/in_memory.rs` (754 lines)
  - [ ] 5.2: `connection/tcp.rs` (477 lines)
