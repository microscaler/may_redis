# Epic 10 — Lint Tightening & Mandatory Rustdocs

**Summary:** Harden the codebase pre-release by tightening `[lints.clippy]` and enforcing mandatory rustdocs on all public interfaces.

**Status:** COMPLETE

## Story Index

| Story | Title | Status |
|-------|-------|--------|
| Story 0 | Epic Overview | ✅ COMPLETE |
| Story 1 | Lint Tightening (Cargo.toml) | ✅ COMPLETE |
| Story 2 | Add # Errors Sections | ✅ COMPLETE |
| Story 3 | Add # Panics Sections | ✅ COMPLETE |
| Story 4 | Final Verification | ✅ COMPLETE |

## What Was Done

1. **Story 1** — Elevated `missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc` from `allow` to `deny` in `Cargo.toml`. This triggered 41 clippy errors across 6 files.

2. **Story 2** — Added `# Errors` sections to 19 public interfaces returning `Result<T, E>`:
   - `client/client.rs` (4 items): `execute_with_timeout`, `execute_timeout`, `execute`, `ping`
   - `client/in_memory.rs` (10 items): InMemoryStore and InMemoryClient methods
   - `client/pipeline.rs` (2 items): `from_responses`, `Pipeline::execute`
   - `connection/tcp.rs` (5 items): All TcpConnector methods
   - `codec/reader.rs` (1 item): `RESPReader::read_value`
   - `core/error.rs` (1 item): `FromRedisValue::from_redis_value`

3. **Story 3** — Added `# Panics` sections to 11 `InMemoryClient` methods that use `.unwrap()` on `Arc<Mutex<>>`.

4. **Story 4** — Verified final state:
   - `cargo clippy --lib --tests --all-features` — **zero warnings, zero errors**
   - `cargo test --lib --all-features` — **341 passed, 0 failed, 28 ignored**
   - `cargo doc --no-deps` — **clean build, zero warnings**
   - `cargo fmt --all --check` — **clean**
   - Fixed broken `ConnectionError` doc links (private module)

## Commit History

| Commit | Message |
|--------|---------|
| `34e1037` | chore(lints): tighten missing_errors_doc, missing_panics_doc, missing_safety_doc to deny |
| `95980d1` | docs: add mandatory # Errors and # Panics sections to all public interfaces |
| `a126877` | docs: fix broken intra-doc links — ConnectionError is in private module |
