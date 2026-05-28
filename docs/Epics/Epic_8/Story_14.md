# Story 8.14 — ToRedisArgs for Unit Type () — Clarify Compile Error

**Objective:** Add `ToRedisArgs` implementation for `()` with a clear error path, OR document why it intentionally doesn't exist. Currently, `()` has no `ToRedisArgs` impl, so `cmd("SET").arg(())` produces a confusing compiler error. This story adds a no-op implementation that produces zero bytes (since `()` has no wire representation) with clear documentation.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** None.

**Source docs:** `docs/redis-implementation-audit.md` (Finding #10, MEDIUM), `src/core/to_args.rs`

## The Problem

`()` has no `ToRedisArgs` implementation. If a user tries:
```rust
cmd("FLUSHDB").arg(())  // pointless but syntactically valid
```

The compiler error is:
```
the trait bound `(): ToRedisArgs` is not satisfied
```

This is confusing because `()` IS the return type of `execute::<()>()` — users might expect it to work both ways.

## Two Approaches

### Approach A: Implement ToRedisArgs for () as no-op

```rust
impl ToRedisArgs for () {
    fn write_redis_args(&self, _buf: &mut Vec<Vec<u8>>) {
        // No-op: unit type has no wire representation
    }
    fn is_simple_arg(&self) -> bool {
        false
    }
}
```

This allows `cmd("FLUSHDB").arg(())` to compile but produce the same wire format as `cmd("FLUSHDB")`. It's a no-op, so it's safe.

### Approach B: No-op with documentation

Keep the current state (no impl) but add a compile-time attribute or a dedicated error module. This is less helpful for users.

**Recommended:** Approach A. The implementation is safe (no bytes written) and prevents the confusing compiler error. It's consistent with Rust's philosophy that `()` is a valid empty type.

## Functional Requirements

1. `impl ToRedisArgs for ()` must write zero bytes to the buffer.
2. `is_simple_arg()` must return `false` (it's not a simple arg since it produces nothing).
3. `cmd("FLUSHDB").arg(())` must produce the same wire format as `cmd("FLUSHDB")` (no extra elements).
4. Documentation must clarify that `()` is a no-op for arguments.

## Non-Functional Requirements

1. **Zero may dependency** — `to_args.rs` has no `may` imports.
2. **No wire impact** — commands with `arg(())` must produce identical output to commands without it.
3. **Backwards compatible** — no existing code uses `arg(())`, so no regression possible.

## Code Anchors

- `src/core/to_args.rs` — add `impl ToRedisArgs for ()`

## Tasks

1. Add `impl ToRedisArgs for ()` to `to_args.rs`.
2. Write unit test: `cmd("FLUSHDB").arg(())` produces `*1\r\n$7\r\nFLUSHDB\r\n`.
3. Write unit test: `cmd("SET").arg("k").arg(())` produces `*2\r\n$3\r\nSET\r\n$1\r\nk\r\n` (no extra arg).
4. Write unit test: `cmd("SET").arg(())` produces `*1\r\n$3\r\nSET\r\n`.

## Unit Test Plan

| Test Name | Input | Expected Wire |
|-----------|-------|---------------|
| `unit_arg_noop_single` | `cmd("FLUSHDB").arg(())` | `*1\r\n$7\r\nFLUSHDB\r\n` |
| `unit_arg_noop_middle` | `cmd("SET").arg("k").arg(())` | `*2\r\n$3\r\nSET\r\n$1\r\nk\r\n` |
| `unit_arg_noop_first` | `cmd("SET").arg(()).arg("v")` | `*2\r\n$3\r\nSET\r\n$1\r\nv\r\n` |
| `unit_arg_noop_multiple` | `cmd("SET").arg(()).arg(())` | `*1\r\n$3\r\nSET\r\n` |
| `unit_arg_noop_is_not_simple` | `()`.is_simple_arg() | `false` |

## Verification Checklist

- [ ] `cargo check --lib` passes
- [ ] `cargo test --lib` — all 238+ tests pass
- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings
- [ ] `cmd("FLUSHDB").arg(())` wire = `cmd("FLUSHDB")` wire (no extra elements)
- [ ] `()` is SimpleArg false
- [ ] No compilation errors with `arg(())` in any position
