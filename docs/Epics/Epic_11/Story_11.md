# Story 11.11 — Add TLS (`rediss://`) and auth parsing to `connect_url`

**Objective:** Extend `connect_url` to handle `rediss://` (TLS) URLs and embedded authentication credentials (`redis://user:password@host:port/db`).

**Epic:** 11 — Code Review Remediation

**Dependencies:** Epic 11.0 (Epic overview)

**Source docs:** `docs/code-review-2026-05-28.md` (Finding CL4, INFO)

**Finding:** CL4 — `connect_url` strips "redis://" prefix but doesn't handle `rediss://` (TLS) or authentication in the URL (`user:password@host:port/db`). Feature gap for future consideration.

## Functional Requirements

- [ ] Parse `rediss://` URLs in addition to `redis://`
- [ ] Parse `redis://user:password@host:port/db` format — extract user, password, host, port, and optional database number
- [ ] Return appropriate error if the URL scheme is neither `redis://` nor `rediss://`
- [ ] The TLS handling is a placeholder for now (return `RedisError::Other("TLS not yet supported")`) — the URL parsing must work regardless

## Non-Functional Requirements

- [ ] URL parsing must follow RFC 3986 — use `url` crate or manual parsing
- [ ] Backwards compatible: `redis://host:port` without auth must still work
- [ ] Invalid port numbers, missing host, or malformed credentials must return descriptive `RedisError::Parse` errors

## Code Anchors

- `src/client/client.rs:54-62` — The current `connect_url` implementation

## Tasks

1. Add `url` crate dependency to `Cargo.toml` (if not already present)
2. Implement `url::Url::parse()` for the URL string
3. Check scheme: allow only `redis://` and `rediss://`
4. Extract host from URL host part
5. Extract port from URL port part (default 6379 if missing)
6. Extract password from URL credentials if present
7. Extract database from URL path if present (e.g., `/12`)
8. Add TLS placeholder (return error with clear message)
9. Call `AUTH` after connection if credentials are provided

## Verification

### Unit Tests

- [ ] Test `redis://localhost:6379` — extracts host="localhost", port=6379
- [ ] Test `redis://localhost` — extracts host="localhost", port=6379 (default)
- [ ] Test `redis://user:pass@localhost:6380/2` — extracts all parts
- [ ] Test `rediss://secure.example.com:6380` — extracts host, port, scheme=TLS
- [ ] Test `redis://localhost:6379` without trailing path — no database number
- [ ] Test `http://localhost:6379` — returns error for unknown scheme
- [ ] Test `not_a_url` — returns parse error
- [ ] Test `redis://localhost:abc` — returns parse error for invalid port

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests

- [ ] Test `connect_url("redis://127.0.0.1:6379")` still works
- [ ] Test `connect_url("redis://127.0.0.1:6379/0")` connects to database 0
- [ ] TLS path returns clear error (not a panic or cryptic failure)

### Expected Results

- `connect_url` parses `redis://` and `rediss://` URLs
- Authentication credentials are extracted
- TLS is acknowledged with a clear "not yet supported" error
- All existing URL callers continue to work
- Clippy clean, all tests pass
