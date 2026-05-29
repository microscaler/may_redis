---
title: SSRF Protection
created: 2026-06-01
updated: 2026-06-01
type: concept
tags: [security, architecture, redis, client]
sources: [raw/may-redis-story3-issues.md]
---

# SSRF Protection

> Server-Side Request Forgery prevention for may-redis connections.

## Overview

SSRF protection blocks connections to internally-resolvable IP addresses (private, link-local, loopback, reserved) after DNS resolution. This prevents attackers from using a may-redis connection to reach internal services (metadata endpoints, internal databases, etc.).

## Architecture

The SSRF guard lives in two layers:

### 1. Standalone Function (`connection/tcp.rs`)

```rust
pub fn ssrf_allowed(addr: &SocketAddr, config: &SsrfConfig) -> bool
```

Takes a resolved `SocketAddr` and `SsrfConfig`, returns `true` if the address is NOT in any deny-listed range.

### 2. Connection Constructor (`connection/connection.rs`)

```rust
pub fn connect_with_ssrf_protection(
    host: &str,
    port: u16,
    timeout: Duration,
    ssrf_config: SsrfConfig,
    command_policy: CommandPolicy,
) -> Result<Self, RedisError>
```

Wires `SsrfConfig` through the `Connection` struct and enforces it during TCP connect.

## Deny-Listed IP Ranges (NFR-013)

All checks are O(1) bitwise range comparisons:

| Range | CIDR | Description |
|-------|------|-------------|
| Private | 10.0.0.0/8 | RFC 1918 |
| Private | 172.16.0.0/12 | RFC 1918 |
| Private | 192.168.0.0/16 | RFC 1918 |
| Link-local | 169.254.0.0/16 | Cloud metadata |
| Loopback | 127.0.0.0/8 | Localhost |
| IPv6 loopback | ::1 | Localhost |
| Unspecified | 0.0.0.0/8 | Reserved |
| Carrier NAT | 100.64.0.0/10 | IANA reserved |
| Multicast | 224.0.0.0/4 | Multicast |
| Reserved | 240.0.0.0/4 | Reserved |
| IPv6 multicast | All | Always blocked |
| IPv6 unspecified | All | Always blocked |

## SsrfConfig

```rust
pub struct SsrfConfig {
    pub deny_private: bool,    // Default: true
    pub deny_link_local: bool, // Default: true
    pub deny_loopback: bool,   // Default: false (backward compat)
}
```

All fields default to `true` except `deny_loopback` (set to `false` for backward compatibility with existing tests and deployments that use localhost Redis).

## Requirements

- **FR-025**: Standalone SSRF guard function — implemented in `tcp.rs`
- **FR-027**: SSRF-aware connection constructor — implemented in `connection.rs`
- **FR-028**: Connection error variant for SSRF violations — `ConnectionError::SsrfViolation(String)`
- **NFR-013**: O(1) bitwise range comparisons — no list iteration

## Related

- [[command-policy]] — Complementary security layer: command-level, not connection-level
- [[may-redis-epic-7-story-1]] — Epic 7 story 3 (connection layer hardening)
