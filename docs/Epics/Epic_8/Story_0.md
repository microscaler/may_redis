# Epic 8 — Implementation Gaps & Hardening

**Objective:** Fix the four critical/medium findings from the redis-implementation-audit so that may-redis is usable by downstream consumers (sesame-idam). This epic addresses: missing basic type conversions, dead dependencies, and connection robustness.

**Epic:** 8 — Implementation Gaps & Hardening
**Dependencies:** Epic 0 through Epic 7 must all pass (`cargo test --lib` zero failures, clippy zero warnings).

**Source docs:** `docs/redis-implementation-audit.md`, `docs/Epics/Epic_0/Story_0.md`

## Rationale

Sesame-IDAM needs to call `client.execute::<String>(client.get("key"))` and `client.execute::<i64>(client.incr("counter"))` — but the current `FromRedisValue` trait only implements `Vec<String>`, `Vec<i64>`, `Vec<RedisValue>`, `Option<String>`, and `usize`. Simple types like `String`, `i64`, `bool`, and `()` do not exist. This makes the entire typed API unusable.

The audit also found dead dependencies (`serde`, `serde_json`) and a missing connection timeout. These are blocking issues for production use.

## Crate Architecture

Work spans four files:

- `src/core/from_value.rs` — FromRedisValue implementations (Story 8.1)
- `src/core/to_args.rs` — ToRedisArgs implementations (Story 8.2)
- `src/connection/connection.rs` — Connection timeout (Story 8.3)
- `Cargo.toml` — Remove unused dependencies (Story 8.4)

## Implementation Order

1. **Story 1: FromRedisValue for basic types** — Foundation. Without this, the typed API is unusable.
2. **Story 2: ToRedisArgs for remaining types** — Complements Story 1, adds bool and Vec<&str>.
3. **Story 3: Connection timeout** — Non-blocking connect with may timer.
4. **Story 4: Remove unused dependencies** — Clean up Cargo.toml.

## Non-Functional Requirements

1. **RESP2 only** — All type conversions handle RESP2 wire format. RESP3 types are out of scope.
2. **Zero may dependency in core** — `from_value.rs` and `to_args.rs` have no `may` imports.
3. **Consistent naming** — Method names follow `redis` crate conventions.
4. **Test coverage** — Every new `FromRedisValue`/`ToRedisArgs` impl has unit tests.
5. **No new dependencies** — No crates added to Cargo.toml.
6. **Backwards compatible** — Existing `FromRedisValue` and `ToRedisArgs` impls unchanged.

## Risks

- **Type inference ambiguity** — Implementing `FromRedisValue` for `String` and `Option<String>` together could cause coherence issues. Need to verify.
- **Connection timeout interaction with epoll** — Adding a timeout to the non-blocking connect path must not interfere with the existing epoll loop.
- **serde removal side effects** — Verify serde is not transitively required by any feature flag or test dependency.
