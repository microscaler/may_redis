# Story 9.2 — Bounded Allocation in CommandBuilder

**Objective:** Replace `Vec::new()` allocations in `CommandBuilder::arg()` and `CommandBuilder::args()` with pre-allocated buffers for the common case.

**Epic:** 9 — JSF-AV Compliance Hardening
**Dependencies:** Epic 9 Story 1 (no-panic pipeline deserialization).

**Source docs:**
- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — "No malloc after init" → "No heap in the hot path"
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — "Stack-Allocated Collections: SmallVec for path/query params"
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — Per-request allocation inventory

## The Problem

```rust
// builder.rs:34 — in arg()
let mut buf = Vec::new();
val.write_redis_args(&mut buf);

// builder.rs:48 — in args()
let mut buf = Vec::new();
for item in vals {
    item.write_redis_args(&mut buf);
}
```

Each call to `.arg()` allocates a new `Vec<Vec<u8>>`. For `SET key value`, that's 3 allocations.

## Functional Requirements

1. Replace `Vec::new()` in `arg()` and `args()` with a reusable buffer owned by `CommandBuilder`.
2. `CommandBuilder` owns a single `Vec<Vec<u8>>` buffer that gets cleared and reused.
3. Wire format must be identical to pre-change output.

## Non-Functional Requirements

1. **No new dependencies.**
2. **Zero may dependency.**
3. **Backwards compatible** — public API unchanged.

## Code Anchors

- `src/protocol/builder.rs` — `CommandBuilder::arg()` (line 32), `CommandBuilder::args()` (line 44), struct definition (line 12)

## Implementation

Add a reusable buffer to `CommandBuilder`:
```rust
pub struct CommandBuilder {
    args: Vec<RedisValue>,
    buf: Vec<Vec<u8>>,  // reusable allocation
}
```

In `new()`, allocate `buf` once. In `arg()`, clear and reuse `buf`. In `clone()`, share the same buffer or create a new one.

## Unit Test Plan

| Test Name | Scenario | Expected |
|-----------|----------|----------|
| `test_cmd_set_key_value` | `cmd("SET").arg("k").arg("v")` | Same wire format |
| `test_cmd_get_key` | `cmd("GET").arg("key")` | Same wire format |
| `test_cmd_mset` | `cmd("MSET").args(&["k1","v1"])` | Same wire format |
| `test_cmd_buffer_reuse` | Multiple `cmd()` calls | No new Vec allocations per call |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all tests pass (wire format identical)
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] Wire format identical to pre-change output (golden tests pass)
- [ ] No `Vec::new()` in production `arg()` / `args()` paths
