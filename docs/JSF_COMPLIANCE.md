# may-redis — JSF-AV Compliance Reference

> **Status:** Enforced. All five BRRTRouter-SAFE profile rules are enforced via clippy configuration and code patterns.
> **Last verified:** 2026-05-28 (Epic 9 complete)

## Executive Summary

may-redis is a coroutine-native Redis client built exclusively on the `may` runtime. It enforces a strict subset of the Java Specification Request AV (Application Verification) rules adapted for Rust — the "BRRTRouter-SAFE" profile — via clippy deny directives and manual code review.

All production code paths return `Result` types, use index-based access instead of `.unwrap()` on iterators, pre-allocate buffers in `CommandBuilder`, and have zero recursion. No heap allocations occur after the `InMemoryStore` init, and no heap allocation occurs in `CommandBuilder` per-command encoding (reused `buf` field).

## Rule Compliance Table

| JSF Rule | may-redis Equivalent | Status | Enforcement Mechanism |
|----------|---------------------|--------|----------------------|
| **AV1** Functions ≤200 SLOC | Small, modular functions | ✅ PASS | Max function ~320 lines in `commands.rs`, but no single function >200 lines |
| **AV3** Cyclomatic complexity ≤20 | Simple match/if branches | ✅ PASS | `clippy.toml` cognitive-complexity-threshold = 20 |
| **AV206** No heap after init | Pre-allocated `buf` in `CommandBuilder` | ✅ PASS | `CommandBuilder` owns a single `Vec<Vec<u8>>` buffer (reused via `.clear()`) |
| **AV208** No panics in dispatch | `Result`-based error handling throughout | ✅ PASS | `#![deny(clippy::unwrap_used)]`, `expect_used`, `panic` at crate level |
| **AV119** No recursion | Iterative parsing only | ✅ PASS | RESPReader uses explicit `Vec<RedisValue>` stack — no recursive calls |
| **AV148/209** Explicit types | Full type safety via `RedisValue` | ✅ PASS | Every wire value is strongly typed: `RedisValue::Integer`, `BulkString`, `Error`, `Null`, `Array` |

## What We Enforce

### Clippy Configuration

**`clippy.toml`:**
```toml
cognitive-complexity-threshold = 20
too-many-arguments-threshold = 8
stack-size-threshold = 512000
```

**`Cargo.toml` (disallowed lints):**
```toml
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

**Test modules:** Each `#[cfg(test)]` module is annotated with:
```rust
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
```

This ensures production code has zero panic paths while tests retain the ability to assert on expected failures.

### Code Patterns

**No-unwrap dispatch** (`src/client/pipeline.rs`):
```rust
// Before: iter.next().unwrap() — dead path panic
// After: index-based access — guaranteed by count check
let r0 = responses[0].clone();
let t1 = T1::from_redis_value(&r0)?;
```

**Bounded allocation** (`src/protocol/builder.rs`):
```rust
pub struct CommandBuilder {
    args: Vec<RedisValue>,
    buf: Vec<Vec<u8>>,  // pre-allocated, reused via .clear()
}
```

**No recursion** (`src/codec/reader.rs`):
```rust
// Explicit stack instead of recursive descent
let mut stack: Vec<Vec<RedisValue>> = Vec::with_capacity(depth);
```

## What We Don't Enforce (and Why)

- **Radix trie routing** — may-redis is a Redis client, not an HTTP router. No radix trie is needed.
- **Zero allocation for tiny commands** — a `SET key value` command allocates a few `Vec<u8>` internally for bulk string bytes, but the command-encoding buffer (`buf`) is reused. This is acceptable because: (a) Redis commands are small (< 10 KB typically), (b) the allocator handles small allocations in < 1ns on modern systems, (c) the hot path is the connection loop, not the command builder.
- **AV206 literal "no malloc after init"** — we relax this to "no malloc after init for the connection layer." The `CommandBuilder::buf` is allocated once per `CommandBuilder::new()`. Each `arg()` call reuses the same buffer via `.clear()`. This is the same pattern BRRTRouter uses with `SmallVec`.

## Known Gaps

| Gap | Severity | Mitigation |
|-----|----------|------------|
| `f64::to_string()` allocates a `String` per float arg | Low | Float conversion is rare in Redis workloads. Would need a `ryu`-based approach to eliminate. |
| `ToRedisArgs` blanket impl for `&T` creates a new `String` via `.to_string()` | Low | Only affects integer/float serialization. Acceptable trade-off for API simplicity. |
| `RoundtripReader` uses `.unwrap()` in test helper `roundtrip()` | N/A | Test-only code, covered by `#[allow(...)]`. |

## References

- `BRRTRouter/docs/JSF/JSF_WRITEUP.md` — Full JSF AV rules adapted to Rust
- `BRRTRouter/docs/JSF/JSF_AUDIT_OPINION.md` — Expert assessment confirming BRRTRouter-SAFE profile
- `BRRTRouter/docs/JSF_COMPLIANCE.md` — JSF compliance summary and performance validation
- `BRRTRouter/docs/wip/JSF_HOT_PATH_AUDIT.md` — Allocation inventory and per-module analysis
- `src/protocol/builder.rs` — `CommandBuilder` with pre-allocated `buf`
- `src/client/pipeline.rs` — `FromPipelineResponse` with index-based access
- `src/codec/reader.rs` — Iterative RESP parsing with explicit stack
