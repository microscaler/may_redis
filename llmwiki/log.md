# LLM Wiki Log

## [2026-05-27] ingest | bootstrap llmwiki from docs

- Created initial `llmwiki/` structure with SCHEMA.md, index.md, docs-catalog.md, log.md.
- Imported full `docs/**/*.md` inventory (11 files) into docs-catalog.md.
- Created topic stubs for RESP protocol, may coroutine pattern, sesame-IDAM integration, module structure.
- Created reference stubs for codebase entry points and command mapping.
- All pages marked `unverified` — docs are design/planning artifacts; code is a single crate with flat modules, not yet the planned modular workspace.
- Created `/home/casibbald/Workspace/microscaler/may_redis/AGENTS.md` referencing the wiki.

## [2026-05-27] ingest | decompose design docs into 7 epics with granular stories

- Created `docs/Epics/` directory structure with 7 epics:
  - `00-epic-overview.md` — project goal, architecture diagrams, execution rules
  - `epic-0-scaffolding.md` — 4 stories (workspace, modules, lint, docs)
  - `epic-1-base.md` — 4 stories (RedisValue, RedisError, ToRedisArgs, full FromRedisValue)
  - `epic-2-codec.md` — 3 stories (RESPWriter, RESPReader, full RESP2 + roundtrip)
  - `epic-3-protocol.md` — 4 stories (CommandBuilder, Commands trait, Request/Response, integration)
  - `epic-4-connection.md` — 4 stories (TcpConnector, Connection struct, epoll loop, integration)
  - `epic-5-client.md` — 3 stories (RedisClient, Pipeline, InMemoryClient)
  - `epic-6-integration.md` — 4 stories (workspace test pass, concurrency, error handling, migration guide)
- Moved source design docs into epic directories:
  - `docs/04-07` → `docs/Epics/epic-3-protocol/docs/` (protocol/client design)
  - `docs/08-11` → `docs/Epics/epic-0-scaffolding/docs/` (module structure, migration, dependencies)
- Each story includes: code anchors, mermaid diagrams (graph/flowchart/sequence), structured tasks, verification criteria
- Updated AGENTS.md to reference epics instead of raw docs
- Total: 26 granular stories across 7 epics, all independently verifiable

## [2026-05-27] fix | resolved two connection-loop bugs that caused integration tests to hang

- Symptom: every `client::client::tests::test_integration_*` test wedged
  indefinitely on `ms02:~/Workspace/microscaler/may_redis`; unit tests
  (codec, core, protocol, in_memory) all passed cleanly. `cargo test`
  had to be killed by `timeout` / SIGTERM.
- Root cause 1 — `spawn_connection_loop` in
  `src/connection/connection.rs` discarded the `bool` returned by
  `nonblock_read` and hardcoded `read_blocked = false`, so the loop
  never called `stream.wait_io()` and never yielded to may's epoll
  scheduler. The connection-loop coroutine starved its worker and
  test coroutines could not push requests / receive responses.
- Root cause 2 — `decode_responses` in the same file called
  `read_buf.split()` (destructive) and only restored the unconsumed
  tail via `reader.take_buf()` / `read_buf.unsplit(...)` on the error
  paths. On the success path the reader was dropped, throwing away
  every RESP frame after the first one in a batched read. Pipelines
  and any back-to-back commands therefore hung on `rx.recv()` for
  every response after the first.
- Fixes (both in `src/connection/connection.rs`):
  - Capture `nonblock_read`'s result: `match nonblock_read(...) { Ok(blocked) => blocked, Err(e) => ... break }`.
  - On the `decode_responses` success arm, call `read_buf.unsplit(reader.take_buf())` before dispatching the value so the outer `while !read_buf.is_empty()` drains the remaining frames.
- Regression coverage added in `src/connection/connection.rs::tests`:
  - `test_decode_responses_multiple_in_one_buffer` — 4 concatenated responses, all dispatched, buffer drained.
  - `test_decode_responses_multiple_with_partial_trailing` — 2 full responses + partial bulk string; full responses dispatched, partial bytes retained for the next read.
- Verification on `ms02`:
  - `cargo test --features test --no-fail-fast -- --test-threads=1` → 147 passed, 0 failed (previously hung indefinitely).
  - `cargo clippy --lib --tests --all-features` → clean.
- Documentation:
  - Created `llmwiki/topics/connection-loop-pitfalls.md` with full root-cause analysis, before/after snippets, regression tests, and cross-cutting guidance for future changes to `src/connection/connection.rs`.
  - Updated `llmwiki/topics/may-coroutine-pattern.md` — fixed the
    `nonblock_read` signature/docstring (the previous claim that it
    returned "true if more data is available" was the opposite of the
    real semantics) and added a pointer to the pitfalls page.
  - Linked the new page from `llmwiki/index.md`.

## [2026-05-27] docs | enriched rustdocs in src/connection/connection.rs with pitfalls cross-references

- Replaced the file-level `//` comment with a proper `//!` sub-module
  doc covering the may-postgres pattern, a "Fragility warning"
  section, and a 3-step checklist (re-read pitfalls page → diff
  against `may_postgres/src/connection.rs` → run integration tests)
  to follow before changing the loop.
- Per-function rustdoc enrichment with explicit references to
  `llmwiki/topics/connection-loop-pitfalls.md`:
  - `nonblock_read` — new "Return value (critical — do not discard)",
    `# Errors`, and `# History` sections naming Bug 1.
  - `nonblock_write` — symmetric return-value / errors documentation.
  - `process_req` — documents the FIFO ordering invariant.
  - `decode_responses` — new "Buffer contract" section explaining
    `BytesMut::split` destructiveness and the three match-arm
    outcomes, plus an inline code comment at the `unsplit` site
    naming Bug 2.
  - `spawn_connection_loop` — numbered loop-invariant list (5 steps),
    a "Two load-bearing details" section calling out both bugs by
    name, and matching `(1)`/`(2)`/`(3)`/`(4)`/`(5)` inline step
    markers in the loop body.
  - `Request`, `PendingRequest`, `Connection`, `Connection::connect`,
    `Connection::send` — expanded with ownership-flow / lifecycle /
    coroutine-context / non-blocking-contract sections so the public
    API explains its concurrency contract in-source.
- Verification on `ms02`:
  - `cargo clippy --lib --tests --all-features` clean (zero errors,
    zero warnings).
  - `cargo doc --no-deps --lib` — only 2 pre-existing
    `connection::ConnectionError` warnings in `src/client/client.rs`
    remain (untouched by this change).
  - `cargo test --features test -- --test-threads=1` — 147 passed.
