---
title: Command Policy
created: 2026-06-01
updated: 2026-06-01
type: concept
tags: [security, architecture, redis, client]
sources: [raw/may-redis-story3-issues.md]
---

# Command Policy

> Enum-based command-level access control for Redis clients.

## Overview

Command policy enforcement blocks dangerous Redis commands (FLUSHALL, CONFIG, DEBUG, etc.) at build time before they reach the connection loop. Implemented as an `enum` with `HashSet` for O(1) lookups.

## Architecture

### CommandPolicy Enum

```rust
pub enum CommandPolicy {
    /// Allow all commands (default — backward compatible)
    AllowAll,
    /// Deny the listed commands; allow everything else
    DenyCommands(HashSet<String>),
    /// Allow only the listed commands; deny everything else
    AllowCommands(HashSet<String>),
}
```

### Validation Flow

Policy is checked in `RedisClient::execute_with_timeout()` **before** building the command:

```rust
// 1. Validate against policy (fast, no allocation)
cmd.validate_policy(&self.inner.command_policy)?;
// 2. Build RESP bytes (only if policy passed)
let Some(data) = cmd.build() else { /* unreachable — already validated */ };
```

This order prevents unnecessary RESP buffer allocation for blocked commands.

### Factory Methods

```rust
CommandPolicy::allow_all()    // AllowAll variant
CommandPolicy::deny_all()     // DenyCommands with default dangerous list
CommandPolicy::deny_set(&["MYCMD"])  // Custom deny list
CommandPolicy::allow_set(&["GET", "SET"])  // Whitelist mode
```

### Default Deny List (AC-3.13)

| Command | Reason |
|---------|--------|
| FLUSHALL | Wipes all databases |
| FLUSHDB | Wipes current database |
| CONFIG | Arbitrary config modification |
| DEBUG | Arbitrary code execution |
| SLAVEOF | Replication hijacking |
| REPLICAOF | Replication hijacking (Redis 5+) |
| SHUTDOWN | Server termination |
| KEYS | Denial of service (blocking scan) |
| BGSAVE | Resource exhaustion |
| BGREWRITEAOF | Resource exhaustion |

Stored as `LazyLock<HashSet<String>>` to avoid const-init limitations with collections.

## Requirements

- **FR-029**: `CommandPolicy` as enum with `HashSet` — implemented in `builder.rs`
- **FR-030**: `validate_policy()` method — checks before build
- **FR-031**: Client enforcement in `execute_with_timeout()` — pre-build validation
- **FR-032**: `command_name()` accessor — returns command as UTF-8 string
- **NFR-015**: O(1) `HashSet::contains` lookups — no linear scan

## Design Decisions

### Why enum instead of struct?
Boolean flags (`allow_all`, `deny_set`) are an anti-pattern: they don't capture mutual exclusivity. An enum with three variants is self-documenting and prevents invalid states (can't set both allow_all and deny_set).

### Why not block RANDOMKEY?
AC-3.13 specifies KEYS, not RANDOMKEY. The old implementation blocked RANDOMKEY, SCAN, SSCAN, HSCAN, ZSCAN as "scan-heavy" commands, but these are NOT in the required deny set per the specification.

### Case-insensitive matching
Command names are stored and compared in uppercase for case-insensitive matching. `flushall`, `FLUSHALL`, and `FlushAll` all match the same entry.

## Error Handling

Blocked commands return `RedisError::Security(String)` with a descriptive message: `"command 'KEYS' is denied by policy"`.

## Related

- [[ssrf-protection]] — Complementary security layer: connection-level, not command-level
- [[may-redis-epic-7-story-1]] — Epic 7 story 3 (security hardening)
- [[sesame-idam-integration]] — Sesame-IDAM uses only safe commands (no CONFIG, DEBUG, etc.)
