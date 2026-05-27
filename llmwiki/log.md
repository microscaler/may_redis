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

## [2026-05-27] commit | landed connection-loop fix + wiki docs as two commits on main

- `5695836 fix(connection): stop integration tests hanging on PING and pipelines`
  — single-file change to `src/connection/connection.rs` covering both
  bug fixes, both regression tests, and the enriched rustdocs. Body
  documents Bug 1 / Bug 2 root causes and points at the pitfalls page.
- `72005b6 docs(llmwiki): add connection-loop pitfalls topic and correct nonblock_read semantics`
  — wiki side of the change: new `topics/connection-loop-pitfalls.md`,
  corrected `topics/may-coroutine-pattern.md` (the `nonblock_read`
  doc-sketch was reversed), `index.md` link, and earlier `log.md`
  entries from this session.
- Branch `main` is `[ahead 2]` of `origin/main`; not pushed
  (project rule: never push without explicit human authorization).
- Both commits authored as `Charles Sibbald <casibbald@gmail.com>` with
  no `Co-authored-by` trailer.
- Pre-existing uncommitted WIP in `examples/debug_redis.rs` and
  `src/client/client.rs` (the test-harness `SyncFlag` -> `run_may`
  cleanup) was left untouched in the working tree for the human to
  commit separately.
- Removed an `._connection-loop-pitfalls.md` AppleDouble file that had
  leaked into the worktree via NFS-from-macOS before it could be
  committed. Adding `._*` to `.gitignore` would harden this.

## [2026-05-27] docs | wrote canonical docs/architecture.md (uncommitted)

- Created `docs/architecture.md` (~489 lines) as the canonical,
  code-accurate architecture reference, replacing the drift in
  `README.md`'s Architecture section and `docs/Epics/Epic_0/Story_0.md`.
- Drift corrected in the new doc:
  - 6-crate workspace claim -> single crate (per ADR-001).
  - `base` module -> `core` module.
  - `redis_*` prefixed filenames -> bare filenames.
  - Bogus `default = ["connection", "client"]` features ->
    real `default = []`, `test` only.
  - Tokio-style `.block_on()` example -> real `may::run` / `may::go!`
    + `client.execute(client.get(..))` example.
  - Single-arg `connect("127.0.0.1:6379")` -> real
    `connect("127.0.0.1", 6379)`.
  - Dead links to moved/missing docs replaced with live links.
- Sections: goals/non-goals; crate shape with full src/ file tree;
  runtime architecture (mermaid: app coroutines -> mpsc Queue ->
  loop -> spsc senders); end-to-end request sequence diagram;
  numbered 5-step connection-loop breakdown mirroring the new
  rustdoc and surfacing Bug 1 / Bug 2 with cross-references to
  `llmwiki/topics/connection-loop-pitfalls.md`; error handling
  graph (`ConnectionError` vs `RedisError`, fatal-drain semantics);
  full `Commands` trait method table; `Pipeline` tuple shapes;
  real feature-flag table; testing architecture (`run_may`,
  `shared_client`, `--test-threads=1`, multi-value decoder tests
  rule); reference-pattern pointers; explicit "what this doc does
  NOT cover" section pointing to rustdoc / Epics / sesame-IDAM doc.
- All 10 internal links verified to resolve.
- File is untracked in the working tree; not committed yet — pending
  user decision on whether to also wire README / AGENTS / llmwiki
  pointers in the same commit.

## [2026-05-27] docs | replaced README.md with a code-accurate landing page (uncommitted)

- Replaced the entire `README.md` (was 102 stale lines, now 136 lines
  of verified-against-the-code content) with a focused landing-page
  README that delegates deep architecture content to
  `docs/architecture.md`.
- Drift removed from the old README:
  - 6-crate workspace mermaid + "Dependency Chain" diagram
    (contradicted ADR-001 single-crate structure).
  - Workspace Structure table listing `base` (renamed to `core`) and
    five other "crates" that don't exist.
  - Bogus Feature Flags section (`default = ["connection", "client"]`,
    `pool`, `connection`, `client` — real `Cargo.toml` has
    `default = []` and `test` only).
  - `.block_on()` quick-start example (tokio-style API the crate
    doesn't have; explicitly forbidden by project rules).
  - Single-string `RedisClient::connect("127.0.0.1:6379")` signature
    (real API is two-arg `connect(host, port)`).
  - Dead link to `docs/08-module-structure.md` (file was moved into
    `docs/Epics/Epic_0/docs/` during the Epics restructure).
- New sections:
  - Status callout explicitly stating v1 scope vs non-goals.
  - Working Quick start example calling the real
    `may::run` + `may::go` + `client.execute(client.get(..))` API,
    including a Pipeline tuple decode.
  - Building and testing block with all day-to-day commands and the
    `--test-threads=1` integration-test requirement.
  - Short Project Conventions section pointing at `AGENTS.md` for
    the full rule set, with a connection-loop-pitfalls warning
    callout for anyone touching `src/connection/connection.rs`.
  - "Where to read next" 9-row routing table by audience / intent.
- All 9 internal links verified to resolve.
- Both `README.md` (modified) and `docs/architecture.md` (untracked)
  remain uncommitted, pending user decision on whether to commit them
  together vs separately and whether to also wire `AGENTS.md` and
  the `llmwiki/index.md` / `llmwiki/docs-catalog.md` pointers in the
  same commit.

## [2026-05-27] ci | fixed `unit-tests` job running Redis-dependent tests without a Redis service (uncommitted)

- Symptom: GitHub Actions `unit-tests` job failed with
  `Connect("Connection refused (os error 111)")` against
  `127.0.0.1:6379` for all 11 `test_integration_*` tests in
  `src/client/client.rs`. Because the `integration-tests` job is
  gated on `needs: unit-tests`, the whole pipeline died before the
  Redis-equipped job ever ran.
- Root cause: `.github/workflows/ci.yaml` `unit-tests` step ran
  `cargo test --features test --lib -- --test-threads=1` which
  includes the `test_integration_*` family but had no Redis service
  attached. The `integration-tests` job ran the *exact same* command
  with a Redis service, so it was both functionally correct and
  wastefully re-executing every unit test.
- Verified scope before patching: the only Redis-touching tests in
  the whole lib are the 11 `test_integration_*` fns in
  `src/client/client.rs`. `test_connection_connect`,
  `test_connection_send_tags`, `test_connection_id`,
  `test_connection_drop` in `src/connection/connection.rs` all use
  `if let Ok(c) = Connection::connect(..) { ... }` so they no-op
  when Redis is unavailable — safe under either job.
- Fix in `.github/workflows/ci.yaml`:
  - `unit-tests` job → appended `--skip test_integration_` so the
    job runs the 136 unit tests with no Redis dependency.
  - `integration-tests` job → added positional filter
    `test_integration_` before `--` so it runs only the 11 Redis
    tests instead of the full 147.
  - Both edits have inline comments explaining the convention and
    why `--test-threads=1` is mandatory.
- Verified on ms02 (Redis available) by running both commands:
  - `unit-tests` cmd: `136 passed; 0 failed; 11 filtered out`.
  - `integration-tests` cmd: `11 passed; 0 failed; 136 filtered
    out`.
  - Total still 147, all green, distributed exactly as the comments
    claim.
- File modified (still uncommitted on ms02):
  `.github/workflows/ci.yaml`.

## [2026-05-27] fix | resolved all 6 ignored doctests

- All 6 `/// ```ignore` doctests converted to `/// ```no_run` and fixed so they compile.
- `src/client/mod.rs` — added missing `Commands` trait import so `client.get()` compiles
- `src/client/pipeline.rs` — added `Commands` import, replaced `?` with `.unwrap()` (doctest `main` can't use `?`)
- `src/codec/mod.rs` — replaced nonexistent `RESPWriter::encode()` static with actual instance API (`RESPWriter::new()` → `write_simple()` → `take()`)
- `src/core/mod.rs` — fixed wrong type name (`Value` → `RedisValue`), corrected import to `may_redis::RedisValue`
- `src/protocol/builder.rs` — corrected import to `may_redis::cmd`
- `src/protocol/mod.rs` — replaced trait-receiver example (`Commands::get`) with standalone `cmd()` example
- Result: 6/6 doctests passing (compile-only via `no_run`), 0 ignored. Full test suite: 153 passed, 0 failed.
- All files updated in `src/`, wiki log updated.

## [2026-05-27] fix | fix integration test panics on CI runners

- Root cause: `unsafe { spawn(...) }` panics on fresh std threads (CI runners)
  because the may coroutine scheduler hasn't been initialized on that thread.
- Fix: introduced `init_may_runtime()` that calls `config().set_workers(1)`
  once per thread, lazily starting the may scheduler. Switched from unsafe
  spawn to `go!` macro (may crate's documented safe wrapper).
- All 11 integration tests pass, 147 unit tests pass, 6 doc-tests pass.
  Total: 153 passed, 0 failed, 0 ignored.
- Committed as `bac707a`.
