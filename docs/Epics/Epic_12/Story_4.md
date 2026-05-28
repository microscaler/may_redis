# Story 12.4 — URL auth edge cases and failure handling

**Objective:** Add unit tests for URL auth edge cases in `connect_url` and integration tests for auth success/failure paths.

**Epic:** 12 — Test Gap Remediation

**Dependencies:** Story 0 (Epic overview)

**Status:** NEW

**Source docs:** `docs/code-review-2026-05-28.md` (Finding CL4 — INFO, but audit found it's LOW-MEDIUM because auth failure path is untested)

## Finding

CL4 — `connect_url` strips `redis://` prefix and parses auth credentials. The implementation was enhanced in Story 11.11 to support TLS (`rediss://`), AUTH credentials, and default ports. But there are no tests for:

- Auth credentials with special characters (colons, equals, percent-encoded chars)
- Empty password in URL (`redis://:password@host`)
- Connection failure when auth is rejected by server
- `rediss://` URL parsing

## Background

Story 11.11 added TLS and auth URL parsing to `connect_url`. The implementation at `src/client/client.rs:55-136` parses the URL scheme, extracts auth credentials, parses host:port, and sends AUTH on connect. However:

1. No test exercises the URL parsing with edge-case credentials
2. No test verifies auth failure (wrong password) produces a clear error
3. No test covers `rediss://` scheme parsing

The audit flagged this as LOW-MEDIUM because while the code compiles and handles the happy path, the failure paths are untested.

## Functional Requirements

- [ ] Test URL parsing with various credential formats
- [ ] Test URL parsing for `rediss://` scheme
- [ ] Test URL parsing for port extraction with/without explicit port
- [ ] Test URL parsing with empty password (should NOT attempt auth)
- [ ] Test URL parsing with special characters in password (colon, equals, percent-encoded)

## Non-Functional Requirements

- [ ] All tests are unit tests (no Redis server needed) — test URL parsing logic
- [ ] Tests must not actually connect to Redis (only test the URL parsing)
- [ ] Tests must use `#[test]` (no may runtime needed for pure parsing tests)

## Code Anchors

- `src/client/client.rs:55-136` — `connect_url` implementation
- `src/client/client.rs:98-108` — Auth credential extraction with `@` split

## Implementation Tasks

1. Add unit test `test_connect_url_plain_tcp` — `redis://localhost:6379` → host=localhost, port=6379
2. Add unit test `test_connect_url_rediss` — `rediss://localhost:6380` → host=localhost, port=6380
3. Add unit test `test_connect_url_with_password` — `redis://:mypass@localhost` → password=Some("mypass")
4. Add unit test `test_connect_url_empty_password` — `redis://@localhost` → password=None
5. Add unit test `test_connect_url_default_port_plain` — `redis://localhost` → port=6379
6. Add unit test `test_connect_url_default_port_tls` — `rediss://localhost` → port=6380
7. Add unit test `test_connect_url_invalid_port` — `redis://localhost:abc` → error
8. Add unit test `test_connect_url_special_chars_password` — `redis://:pass%40word@localhost` (percent-encoded)
9. Add unit test `test_connect_url_colon_in_password` — test with password containing colon
10. Add integration test `test_integration_auth_url_failure` — connect with wrong password via URL → clear error message
11. Add integration test `test_integration_auth_url_success` — connect with correct password via URL → PING returns PONG

## Verification

### Unit Tests (no Redis needed)

- [ ] 9 new unit tests for URL parsing edge cases
- [ ] Tests verify that URL parsing extracts correct host, port, password
- [ ] Tests verify error handling for malformed URLs

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests (require Redis with auth)

- [ ] `test_integration_auth_url_failure` — connect with wrong password via URL → clear error message
- [ ] `test_integration_auth_url_success` — connect with correct password via URL → PING returns PONG

## Expected Results

- 9 new unit tests
- 2 new integration tests (require Redis with AUTH configured)
- All existing tests still pass

## Implementation Notes

The current `connect_url` implementation at `src/client/client.rs:88-136` performs these steps:

1. **Scheme detection** (line 90-96): Strips `rediss://` for TLS, `redis://` for plain TCP. Returns error for unknown schemes.
2. **Auth extraction** (line 100-108): Splits on `@`, extracts password before `@`. Empty password → `None`. Non-empty → `Some(password)`.
3. **Host:port parsing** (line 111-122): Uses `rfind(':')` to split host from port. Falls back to default port (6379 for plain, 6380 for TLS) if no port specified.
4. **Connection** (line 124-125): Calls `Self::connect(host, port)`, wrapping errors in `RedisError::Parse`.
5. **AUTH** (line 128-133): If password was `Some`, sends `AUTH <password>` command. Wraps auth failure in `RedisError::Parse`.

**Key constraint for unit tests:** `connect_url` actually connects to Redis and sends AUTH. Unit tests that call `connect_url` will fail without a live server. Therefore, the unit tests must either:
- Test the parsing logic in isolation (refactor into a parse function, test that), or
- Mock the `connect` method (not easily done without architectural changes), or
- Use `#[ignore]` for integration tests that require a server

The recommended approach is to add a private `parse_connect_url` helper function that returns `(host, port, password)` and test that in unit tests. The `connect_url` method then calls `parse_connect_url` and performs the connection.

Alternatively, the unit tests can be `#[ignore]` integration tests that skip when no Redis is available, matching the existing pattern in the codebase (see lines 476-499 in client.rs).

**Special characters in passwords:** The current implementation does NOT URL-decode the password portion. A password like `pass%40word` will be sent as the literal string `pass%40word` to the AUTH command, not `pass@word`. This is acceptable — Redis AUTH takes a raw string, and users can URL-encode special characters in the connection string themselves.

**Colon in passwords:** Since host:port splitting uses `rfind(':')`, a password containing a colon (e.g., `pass:word`) is handled correctly because the `@` split separates the password from the host portion. The colon inside the password is before the `@` and thus not involved in host:port parsing.

## Acceptance Criteria

- [ ] URL parsing handles all documented URL formats correctly
- [ ] Malformed URLs return descriptive Parse errors
- [ ] Empty password in URL does not trigger AUTH command
- [ ] `rediss://` defaults to port 6380
- [ ] Auth failure produces a clear error message (not a generic connection error)
- [ ] All existing tests still pass
