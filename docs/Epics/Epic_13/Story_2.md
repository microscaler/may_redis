# Story 2 — URL Parsing and Auth

**Finding IDs:** #3, #4, #5, #18 (CRITICAL/LOW)

**Objective:** Fix URL parsing to correctly handle IPv6, password encoding, and prefix stripping per RFC 3986.

---

## Issue #3: IPv6 URLs Are Not Supported — Broken Host Resolution

**Severity:** CRITICAL

### Problem Description

URL parsing uses `host_part.rfind(':')` to split host and port:
```rust
host_part.rfind(':')
    .map(|colon_idx| {
        let host = &host_part[..colon_idx];
        let port_str = &host_part[colon_idx + 1..];
```

For IPv6 URLs like `redis://[::1]:6379`, `rfind(':')` splits on the **last** colon, producing host=`[::1` (missing closing bracket) and port=`6379`. The host resolver then fails.

For `redis://[2001:db8::1]:6379`, it produces host=`[2001:db8::1` (missing `]`) and port=`6379`.

The same bug exists in `TcpConnector::connect_url` in `tcp.rs`.

### Attack Vector

Users who deploy to IPv6-only environments cannot connect at all. This is not an exploit but a denial of service for legitimate users. An attacker who can cause connection failures (e.g., via DNS pollution) can exploit this by redirecting traffic to IPv6 addresses.

### Acceptance Criteria

1. **AC-2.1:** URLs with IPv6 addresses in bracket notation (`[::1]`, `[2001:db8::1]`) must be parsed correctly.
2. **AC-2.2:** The closing `]` must be included in the host string.
3. **AC-2.3:** If no port is specified, the default port must be used for both IPv4 and IPv6 hosts.
4. **AC-2.4:** URLs without brackets containing a single `:` (e.g., `example.com:6379`) must still work.
5. **AC-2.5:** URLs with no port (e.g., `redis://example.com`) must use the default port.

### Functional Requirements

- **FR-010:** Parse URL using RFC 3986 rules: if host starts with `[`, find the matching `]` before looking for `:`.
- **FR-011:** Implement a helper `parse_host_port(host_part: &str) -> Result<(String, Option<u16>), RedisError>`.
- **FR-012:** Handle three cases: `[ipv6]:port`, `ipv4:port`, `hostname` (no port).
- **FR-013:** Both `RedisClient::connect_url()` and `TcpConnector::connect_url()` must use the same parsing logic.

### Non-Functional Requirements

- **NFR-007:** The parser must handle any valid RFC 3986 authority component.
- **NFR-008:** Parsing must be O(n) where n is the URL length — no exponential backtracking.

---

## Issue #4: `@` in Password Breaks URL Parsing

**Severity:** CRITICAL

### Problem Description

URL parsing uses `find('@')` to locate the first `@`:
```rust
let (password, host_part) = rest.find('@').map_or((None, rest), |idx| {
    let password = &rest[..idx];
    let host_part = &rest[idx + 1..];
```

A password containing `@` (e.g., `redis://myuser:p@ss@host:6379`) would parse password as `myuser:p` and host as `ss@host:6379`. The host parse then fails because `ss@host:6379` is not a valid host:port.

The correct behavior per RFC 3986 is to find the **last** `@` (the userinfo delimiter), not the first.

### Attack Vector

This is primarily a denial-of-service for users with `@` in their passwords. However, in an OAuth/token-based system where access tokens are embedded in URLs (which commonly contain `@`), this would cause all connections to fail.

### Acceptance Criteria

1. **AC-2.6:** Passwords containing `@` must be parsed correctly — find the last `@` in the authority component.
2. **AC-2.7:** A URL with no `@` must still work (no auth).
3. **AC-2.8:** A URL with `:` in the password must work (e.g., `redis://:pass:word@host:6379`).
4. **AC-2.9:** A URL with `@` and `:` in the password must work (e.g., `redis://user:p@ss:word@host:6379`).

### Functional Requirements

- **FR-014:** Replace `find('@')` with `rfind('@')` to find the last `@` in the authority.
- **FR-015:** If the password portion contains `:`, use `rsplit_once(':')` on the authority to separate password from host:port.

### Non-Functional Requirements

- **NFR-009:** Password handling must be constant-time where possible (no early-exit on mismatch) to prevent timing attacks.

---

## Issue #5: Password Is NOT URL-Decoded from URL

**Severity:** CRITICAL

### Problem Description

The password extracted from the URL is used raw. A password containing `%40` (URL-encoded `@`) would be sent as the literal `%40` to Redis, not decoded to `@`. There is no URL decoding anywhere in the codebase.

Combined with Issue #4, there is no way to represent an `@` in a password within a URL. The user must use URL encoding, but the library does not decode it.

### Attack Vector

This prevents legitimate users from connecting with passwords containing special characters (`@`, `:`, `%`, `/`, `?`, `#`, `[`, `]`). In a cloud environment where passwords are auto-generated and may contain any character, this would cause connection failures.

### Acceptance Criteria

1. **AC-2.10:** Passwords must be URL-decoded before being sent to Redis via the AUTH command.
2. **AC-2.11:** Only the password component (not the host) should be URL-decoded.
3. **AC-2.12:** Invalid percent-encoding (e.g., `%GG`) in the password must return a parse error, not crash.

### Functional Requirements

- **FR-016:** Implement a `url_decode(s: &str) -> Result<String, RedisError>` helper.
- **FR-017:** URL-decode the password portion before passing it to `CommandBuilder::new("AUTH").arg(password)`.
- **FR-018:** URL-decode must only decode `%HH` sequences and pass through all other characters unchanged.

### Non-Functional Requirements

- **NFR-010:** URL decoding must be O(n) with no backtracking.

---

## Issue #18: connect_url Double-Prefix Vulnerability

**Severity:** LOW

### Problem Description

```rust
let url = url.strip_prefix("redis://").unwrap_or(url);
```

If the URL is `redis://redis://example.com:6379`, it strips the first prefix to get `redis://example.com:6379`, which is then parsed as host `redis://example.com` and port `6379`. The resolver fails, but the behavior is unpredictable — some DNS resolvers might treat `redis://example.com` as a hostname and look it up, potentially leaking information.

### Attack Vector

An attacker who can control the URL input (e.g., via a web application that accepts Redis connection strings) can cause the client to attempt DNS lookups for arbitrary hostnames. While this doesn't directly cause SSRF (the connection goes to whatever hostname resolves), it's an information leak: the server's DNS resolver will query for the attacker-controlled hostname, potentially revealing internal DNS patterns.

### Acceptance Criteria

1. **AC-2.13:** A double `redis://` prefix must be rejected with a clear error.
2. **AC-2.14:** Only one prefix strip must occur — no recursive stripping.
3. **AC-2.15:** Invalid protocols (e.g., `http://`, `ftp://`) must be rejected with a descriptive error.

### Functional Requirements

- **FR-019:** After stripping `redis://`, check that the remaining string does NOT start with `redis://` and reject if so.
- **FR-020:** Validate the scheme before parsing — only `redis://` and `rediss://` are accepted.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/client/client.rs` lines 98-122: `connect_url` host:port parsing
- `src/connection/tcp.rs` lines 138-167: `connect_url` and `connect_url_timeout`
- RFC 3986: URI generic syntax (Section 3.2: Authority)
