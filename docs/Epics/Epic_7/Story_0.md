# Epic 7 — Redis Command Expansion

**Objective:** Expand may-redis from 22 commands to ~80 commands covering the full Redis command surface. The goal is completeness — every Redis command should be expressible through the `Commands` trait so that callers never need to reach for `cmd()` directly.

**Epic:** 7 — Redis Command Expansion
**Dependencies:** Epic 0 through Epic 6 must all pass (`cargo test --lib` zero failures, clippy zero warnings).

**Source docs:** `docs/01-protocol-analysis.md` (RESP wire format), `docs/Epics/Epic_0/Story_0.md` (project architecture), the command coverage audit table in `docs/08-command-audit.md`.

## Rationale

sesame-idam uses 11 canonical Redis commands, all already implemented. The remaining ~60 commands are standard Redis features needed for:
- General client completeness (MGET, MSET, DECR, SETNX, etc.)
- Hash manipulation (HDEL, HKEYS, HGETALL, etc.)
- Set operations beyond basic add/remove (SMEMBERS, SINTER, SUNION, etc.)
- List operations (LPUSH, RPUSH, LRANGE, etc.)
- Sorted set operations (ZADD, ZRANGE, ZRANK, etc.)
- Pub/Sub subscription (SUBSCRIBE, UNSUBSCRIBE)
- Transactions (MULTI, EXEC, DISCARD, WATCH)
- Server/admin commands (SELECT, TYPE, SCAN, FLUSHALL)
- Scripting (EVAL, EVALSHA)

sesame-idam is NOT currently using these. This epic is for client completeness and future-proofing.

## Crate Architecture

All work is in the `Commands` trait in `src/protocol/commands.rs`. No new modules needed. Commands follow the existing pattern:

```rust
/// DESCRIPTION
#[must_use = "call .build() to encode the command"]
fn command_name<K: ToRedisArgs>(&self, key: K) -> CommandBuilder {
    CommandBuilder::new("COMMAND_NAME").arg(key)
}
```

Each command also needs a unit test in the `mod tests` section of `commands.rs`.

## Implementation Order

Commands are grouped by data type and dependency. Each group is an independent story (Story 1-7). The order matters because some commands share implementation patterns:

1. ~~**Story 1: String Extension**~~ — ✅ **COMPLETE** — DECR, DECRBY, SETNX, MGET, MSET, MSETNX, STRLEN, GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, BITCOUNT_RANGE
2. ~~**Story 2: Hash**~~ — ✅ **COMPLETE** — HDEL, HDEL_FIELDS, HKEYS, HGETALL, HMSET, HINCRBY, HLEN, HEXISTS, HSCAN, HSCAN_MATCH
3. ~~**Story 3: Set**~~ — ✅ **COMPLETE** — SMEMBERS, SPOP, SPOP_COUNT, SRANDMEMBER, SRANDMEMBER_COUNT, SCARD, SINTER, SUNION, SMOVE, SSCAN, SSCAN_MATCH
4. ~~**Story 4: List**~~ — ✅ **COMPLETE** — LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP
5. ~~**Story 5: Sorted Set**~~ — ✅ **COMPLETE** — ZADD, ZADD_MULTI, ZREM, ZREM_MEMBERS, ZRANGE, ZRANGE_WITHSCORES, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMAX_COUNT, ZPOPMIN, ZPOPMIN_COUNT, ZSCAN, ZSCAN_MATCH, ZRANGEBYSCORE, ZRANGEBYSCORE_WITHSCORES, ZRANGEBYSCORE_LIMIT
6. ~~**Story 6: Pub/Sub + Transactions**~~ — ✅ **COMPLETE** — SUBSCRIBE, UNSUBSCRIBE, UNSUBSCRIBE_CHANNELS, PSUBSCRIBE, PUNSUBSCRIBE, PUNSUBSCRIBE_PATTERNS, MULTI, EXEC, DISCARD, WATCH, UNWATCH
7. ~~**Story 7: Server/Admin**~~ — ✅ **COMPLETE** — SELECT, TYPE, MOVE, RENAME, RENAMENX, SORT, SORT_LIMIT, SORT_LIMIT_ORDER, SCAN, SCAN_MATCH, TOUCH, SAVE, BGSAVE, FLUSHALL, PTTL, PEXPIRE, PEXPIREAT, PERSIST, SHUTDOWN, SHUTDOWN_NOSAVE, INFO, INFO_SECTION, CONFIG_GET

## Verification Checklist

- [x] All new commands compile with `cargo check --lib`
- [x] All commands have unit tests verifying RESP wire encoding
- [x] `cargo test --lib` passes with zero failures
- [x] `cargo clippy --lib --tests --all-features -- -D warnings` passes with zero warnings
- [x] Stories 1 and 2 in Story_0.md are marked COMPLETE
- [x] Commands are discoverable via `Commands` trait (not just `cmd()` builder)
- [x] Every command has a corresponding `test_command_<name>_encoding()` unit test

## Non-Functional Requirements

1. **RESP2 only** — All commands encode/decode RESP2 wire format. RESP3 types are out of scope.
2. **Zero may dependency in protocol** — `commands.rs` only depends on `core` (ToRedisArgs) and `builder` (CommandBuilder). No `may` imports.
3. **Consistent naming** — Method names follow `redis` crate conventions (snake_case, lowercase). Commands use uppercase RESP names.
4. **Must-use attribute** — Every method has `#[must_use = "call .build() to encode the command"]`.
5. **Test coverage** — Every command has at least one unit test verifying the RESP wire encoding. Tests in `mod tests` at the bottom of `commands.rs`.
6. **No new dependencies** — No crates added to Cargo.toml. All implementation is within the existing crate.
7. **In-memory support** — InMemoryClient does NOT need to implement every command. It only implements the commands sesame-idam actually uses. New commands can remain unimplemented in InMemoryClient for now.

## Risks

- **Command builder complexity** — Some commands (ZRANGEBYSCORE, SORT, EVAL) have many optional arguments. Start with the simplest form, add variadic args later if needed.
- **Response types** — Commands like SUBSCRIBE/UNSUBSCRIBE return different RESP types. Response handling in the connection layer may need updates.
- **Testing without Redis** — Admin commands (SAVE, SHUTDOWN, BGSAVE) should only be tested via wire encoding, not against a live server.
- **RESP2 vs RESP3** — Some commands have different RESP3 wire formats (e.g., MAP, SET types). Stick to RESP2 for this epic.
