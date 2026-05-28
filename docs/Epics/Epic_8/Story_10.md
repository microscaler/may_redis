# Story 8.10 — Add Array Depth Limit in RESPReader to Prevent Stack Overflow

**Objective:** Add a maximum depth limit for nested arrays in `RESPReader::read_array()` to prevent stack overflow from deeply nested or malicious RESP responses. A malicious server could send `*1\r\n*1\r\n*1\r\n...` causing unbounded recursion in `read_value()` → `read_array()` → `read_value()`.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None. Pure code change in codec module.

**Source docs:** `docs/redis-implementation-audit.md` (Finding #6, HIGH — DoS vector), `src/codec/reader.rs` lines 156-173

## The Problem

`read_array()` is recursive via `read_value()`:

```rust
fn read_array(&mut self) -> Result<RedisValue, RedisError> {
    let len = ...parse array header...;
    for _ in 0..len {
        items.push(self.read_value()?);  // calls read_array() recursively
    }
}
```

`read_value()` calls `read_array()`, which calls `read_value()`. No depth limit is enforced. Rust stack depth is ~8MB on Linux, allowing roughly 40,000 levels of recursion before stack overflow.

**Impact:** A malicious or buggy Redis server (or a man-in-the-middle) can crash the client process via stack overflow. This is a denial-of-service vulnerability.

## Functional Requirements

1. `RESPReader` must track recursion depth during `read_value()` calls.
2. Maximum depth: 256 levels (arbitrary but sane — Redis admin commands rarely nest more than a few levels).
3. When depth exceeds 256, return `Err(RedisError::Parse("array nesting depth exceeds maximum of 256"))`.
4. The depth counter must be thread-safe per-reader (not shared across readers).

## Non-Functional Requirements

1. **Zero may dependency** — codec has no `may` imports.
2. **No performance regression** — depth counter is a simple `usize` field, incremented/decremented on each entry/exit.
3. **Error is descriptive** — must mention "depth" or "nesting" so users understand the issue.
4. **Configurable** — if a user legitimately needs deeper nesting, they should be able to configure it via `RESPReader::with_max_depth()`.

## Code Anchors

- `src/codec/reader.rs` — `RESPReader` struct, `read_value()`, `read_array()`

## Tasks

1. Add `max_depth: usize` field to `RESPReader` (default 256).
2. Add `current_depth: usize` field (or pass as argument in recursive calls).
3. Modify `read_array()` to check depth before recursing.
4. Add `RESPReader::with_max_depth(depth: usize)` builder method.
5. Write unit tests for depth limit enforcement.

## Unit Test Plan

| Test Name | Input | Expected |
|-----------|-------|----------|
| `depth_ok_10` | 10-level nested array `*1\r\n*1\r\n...` | `Ok(RedisValue::Array(...))` |
| `depth_exactly_256` | 256-level nested array | `Ok(...)` (at limit, not over) |
| `depth_257_exceeds` | 257-level nested array | `Err(Parse("exceeds maximum of 256"))` |
| `depth_custom_50` | `RESPReader::with_max_depth(50)` → 51 levels | `Err(Parse(...))` |
| `depth_zero` | `RESPReader::with_max_depth(0)` → any array | `Err(Parse(...))` (no arrays allowed) |
| `depth_flat_arrays` | Multiple sibling arrays `*2\r\n$5\r\nhello\r\n$3\r\nbar\r\n` | `Ok(...)` (siblings, not nested) |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] 256-level nesting succeeds, 257 fails
- [ ] Flat arrays (siblings) are unaffected by depth limit
- [ ] `RESPReader::with_max_depth(0)` correctly rejects all arrays
- [ ] Error message is clear and actionable
