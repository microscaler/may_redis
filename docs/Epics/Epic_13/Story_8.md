# Story 8 — Comprehensive Fuzzing and Property Tests

**Finding IDs:** All remaining coverage gaps

**Objective:** Add fuzzing infrastructure and property-based tests to catch edge cases that manual audits miss. This is the final story — it depends on all other stories being stable.

---

## Problem Statement

Manual security audits find known-vulnerability patterns but miss emergent bugs: corner cases, encoding edge cases, and protocol-level ambiguities. Property-based testing and fuzzing are needed to exercise the code paths that manual tests don't cover.

### What's Missing

1. **No fuzzing of RESP parser** — malformed RESP is the primary attack surface
2. **No property tests for URL parser** — the URL parsing bugs in Story 2 could have been caught by a simple property test
3. **No fuzzing of the command builder** — arbitrary command names and argument combinations
4. **No coverage-guided fuzzing** — no AFL++ or libFuzzer integration
5. **No roundtrip property tests** — encode → decode should yield the same value for all valid inputs
6. **No negative fuzzing** — feed garbage bytes and verify graceful failure (no panics, no UB)

---

## Task 1: RESP Reader Fuzzing

### Objective

Fuzz the `RESPReader` with malformed inputs to find:
- Panics on arbitrary byte sequences
- Out-of-bounds reads
- Buffer overflows
- Infinite loops on malformed input
- Memory corruption from crafted RESP

### Acceptance Criteria

1. **AC-8.1:** Add a `cargo fuzz` target using `cargo-fuzz` (libFuzzer).
2. **AC-8.2:** The fuzz target must cover all RESP type markers: `+`, `-`, `:`, `$`, `*`.
3. **AC-8.3:** The fuzz target must generate:
   - Valid RESP values (positive control)
   - Truncated values (missing CRLF, incomplete bulk strings)
   - Negative lengths (e.g., `$-999`)
   - Zero-length arrays with children (e.g., `*0\r\n$1\r\nx\r\n`)
   - Over-depth nesting (e.g., `***[...1000 levels...]`)
   - Non-UTF8 bulk string content
   - Invalid integer formats (e.g., `:1.5`, `:+`, `:`)
   - Arbitrary binary data as RESP markers
4. **AC-8.4:** The fuzz target must run for 60 seconds in CI and must not find any panics or hangs.
5. **AC-8.5:** A corpus directory must be maintained with seed inputs for all edge cases.

### Functional Requirements

- **FR-070:** Create `src/fuzz/responder.rs` fuzz target.
- **FR-071:** Create `src/fuzz/command_builder.rs` fuzz target.
- **FR-072:** Create `src/fuzz/url_parser.rs` fuzz target.
- **FR-073:** Each fuzz target must use `no_main` and `libfuzzer-sys` entry points.
- **FR-074:** Fuzz targets must be excluded from regular `cargo test` (only built with `cargo fuzz`).

### Non-Functional Requirements

- **NFR-037:** Fuzz targets must be gated behind a `fuzzing` feature flag.
- **NFR-038:** Fuzz targets must not link against the full may runtime (they run in a single-threaded libFuzzer context).

---

## Task 2: Property-Based Tests

### Objective

Add `proptest`-based property tests that verify invariants hold for all inputs.

### Properties to Test

**RESP Roundtrip:**
- For every `RedisValue` that fits within size limits, `encode(encode(v)) == encode(v)` (RESP encoding is deterministic)
- For every valid RESP byte sequence that parses successfully, `decode(encode(decode(raw))) == decode(raw)` (encode/decode roundtrip)

**URL Parsing:**
- For every valid `redis://` URL, `parse(connect_url(parse(url))) == parse(url)` (roundtrip through connect)
- For every password string P, `parse("redis://user:" + url_encode(P) + "@host:6379").password == P`

**Command Builder:**
- For every sequence of arguments, `cmd("TEST").args(args).len() == 1 + args.len()`
- For every argument, `cmd("TEST").arg(a).build()` produces valid RESP

**InMemoryClient:**
- For every key K, `get(set(K, V)) == V` (commutativity of set/get)
- For every key K, `del(get(K)) == true` when K exists
- For every key K not in store, `get(K) == None` (null semantics)

**FromRedisValue:**
- For every `RedisValue::Integer(n)`, `from_redis_value(to_redis_value(n)) == n` (roundtrip)
- For every overflow boundary (i32::MAX+1, i32::MIN-1), `from_redis_value` returns `Err`
- For every `RedisValue::BulkString(b)`, `from_redis_value::<String>` succeeds iff b is valid UTF-8

### Acceptance Criteria

1. **AC-8.6:** Add `proptest` to dev-dependencies.
2. **AC-8.7:** Create `src/core/property_tests.rs` with property tests for all types in `FromRedisValue`.
3. **AC-8.8:** Create `src/codec/property_tests.rs` with property tests for RESP encoding/decoding.
4. **AC-8.9:** Create `src/connection/property_tests.rs` with property tests for InMemoryClient.
5. **AC-8.10:** Property tests must run for 100 iterations (default proptest) and pass in CI.

### Functional Requirements

- **FR-075:** Implement `Arbitrary` for `RedisValue` — generate random valid Redis values with bounded size.
- **FR-076:** Implement `Arbitrary` for `Vec<u8>` — generate random byte sequences of bounded length.
- **FR-077:** Implement `Arbitrary` for `String` — generate valid UTF-8 strings of bounded length.
- **FR-078:** Property tests must use `proptest!` macro with descriptive names.

### Non-Functional Requirements

- **NFR-039:** Property tests must not depend on a running Redis server.
- **NFR-040:** Property tests must run in <= 5 seconds total.

---

## Task 3: Negative Fuzzing (Panic Hunt)

### Objective

Deliberately feed the parser garbage bytes to verify that:
- No panics occur
- All errors are `RedisError::Parse` (not unwraps)
- Buffer state is preserved after errors (no corruption)

### Acceptance Criteria

1. **AC-8.11:** Create a `negative_fuzz` test that feeds random byte sequences to `RESPReader::read_value()`.
2. **AC-8.12:** The test must verify that every random input either returns `Ok(_)` or `Err(RedisError::Parse(_))`.
3. **AC-8.13:** The test must verify that NO call to `read_value()` with garbage input panics.
4. **AC-8.14:** After any error, the reader's buffer must be in a valid state — subsequent reads must not leak data from previous reads.
5. **AC-8.15:** Run 10,000 random byte sequences (each 1-1024 bytes) and verify all pass.

### Functional Requirements

- **FR-079:** Create `#[test] fn negative_fuzz_resp_reader()` in `src/codec/reader.rs`.
- **FR-080:** Create `#[test] fn negative_fuzz_url_parser()` in `src/connection/tcp.rs`.
- **FR-081:** Create `#[test] fn negative_fuzz_command_builder()` in `src/protocol/builder.rs`.
- **FR-082:** Each test must use `fastrand` to generate random inputs.

### Non-Functional Requirements

- **NFR-041:** Negative fuzz tests must run in <= 10 seconds total.
- **NFR-042:** Negative fuzz tests must not allocate more than 1MB of memory total.

---

## Task 4: Fuzzing Infrastructure for CI

### Objective

Ensure fuzzing runs in CI as a periodic (not blocking) check.

### Acceptance Criteria

1. **AC-8.16:** Fuzz targets must compile in CI (via `cargo fuzz build`).
2. **AC-8.17:** A CI job must run fuzz targets for 60 seconds each on a schedule (daily, not per-PR).
3. **AC-8.18:** Fuzz crashes must be reported to the project maintainers.
4. **AC-8.19:** Corpus files must be committed to the repository for reproducibility.

### Functional Requirements

- **FR-083:** Add a `fuzz` entry in the project's CI configuration.
- **FR-084:** Document how to run fuzz targets locally: `cargo fuzz run responder`.
- **FR-085:** Create a `fuzz/corpus/` directory with seed inputs.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
cargo fuzz build          # verify fuzz targets compile
cargo fuzz run responder -- -max_total_time=60  # verify fuzzing works
```

## Source References

- `src/codec/reader.rs` — RESP parsing (fuzz target)
- `src/codec/writer.rs` — RESP encoding (roundtrip property test)
- `src/client/client.rs` — URL parsing (property test)
- `src/protocol/builder.rs` — command builder (property test)
- `src/client/in_memory.rs` — InMemoryClient (property test)
- `src/core/from_value.rs` — FromRedisValue (property test)
