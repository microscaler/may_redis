# may-redis

A coroutine-native Redis client built on the
[`may`](https://crates.io/crates/may) stackful-coroutine runtime.

**Zero tokio. Zero `async` / `.await`. Only `may` coroutines.**

---

## v0.1.0 — Initial Stable Platform

Released 2026-06-06. This release captures a fully tested codebase with:
- 620 passing tests (0 failures) across unit, integration, E2E, and perf suites
- Docker-managed test fixtures via bollard (no hardcoded `localhost:6379`)
- Complete RESP2 codec with roundtrip verification
- Full `Commands` trait implementation (~96 tested commands across 9 categories)
- Redis Cluster support (CRC16 slot routing, MOVED/ASK redirects, topology)
- TLS/mTLS support (rustls 0.23)
- SSRF protection and command policy enforcement
- JSF-AV compliance hardening

---

## Redis API coverage

`may-redis` implements the vast majority of the Redis command set with
correct RESP2 encoding tests. Every command is tested to produce the
exact wire format the Redis server expects.

**96 commands tested across 9 data categories — 0 gaps in trait methods**

<details>
<summary>Click to expand full command coverage table</summary>

| Category | Commands |
|----------|----------|
| **STRING** | `GET` (`get`), `SET` (`set`, `set_ex`), `SETEX` (`setx`), `SETNX` (`setnx`), `MGET` (`mget`), `MSET` (`mset`), `MSETNX` (`msetnx`), `DEL` (`del`), `EXISTS` (`exists`), `APPEND` (`append`), `STRLEN` (`strlen`), `GETRANGE` (`getrange`), `SETRANGE` (`setrange`), `SETBIT` (`setbit`), `GETBIT` (`getbit`), `BITCOUNT` (`bitcount`, `bitcount_range`), `INCR` (`incr`), `INCRBY` (`incrby`), `DECR` (`decr`), `DECRBY` (`decrby`), `TTL` (`ttl`), `PEXPIRE` (`pexpire`), `PEXPIREAT` (`pexpireat`), `PERSIST` (`persist`), `MOVE` (`move_key`) |
| **HASH** | `HSET` (`hset`), `HGET` (`hget`), `HMSET` (`hmset`), `HDEL` (`hdel`, `hdel_fields`), `HGETALL` (`hgetall`), `HKEYS` (`hkeys`), `HLEN` (`hlen`), `HEXISTS` (`hexists`), `HINCRBY` (`hincrby`), `HSCAN` (`hscan`, `hscan_match`) |
| **SET** | `SADD` (`sadd`), `SISMEMBER` (`sismember`), `SREM` (`srem`), `SMEMBERS` (`smembers`), `SPOP` (`spop`, `spop_count`), `SRANDMEMBER` (`srandmember`, `srandmember_count`), `SCARD` (`scard`), `SINTER` (`sinter`), `SUNION` (`sunion`), `SMOVE` (`smove`), `SSCAN` (`sscan`, `sscan_match`) |
| **LIST** | `LPUSH` (`lpush`), `RPUSH` (`rpush`), `LPOP` (`lpop`), `RPOP` (`rpop`), `LLEN` (`llen`), `LRANGE` (`lrange`), `LINDEX` (`lindex`), `LSET` (`lset`), `LREM` (`lrem`), `LTRIM` (`ltrim`), `BLPOP` (`blpop`), `BRPOP` (`brpop`) |
| **SORTED SET** | `ZADD` (`zadd`, `zadd_multi`), `ZCARD` (`zcard`), `ZCOUNT` (`zcount`), `ZINCRBY` (`zincrby`), `ZPOPMAX` (`zpopmax`, `zpopmax_count`), `ZPOPMIN` (`zpopmin`, `zpopmin_count`), `ZRANGE` (`zrange`, `zrange_withscores`), `ZRANGEBYSCORE` (`zrangebyscore`, `zrangebyscore_withscores`, `zrangebyscore_limit`), `ZRANK` (`zrank`), `ZREM` (`zrem`, `zrem_members`), `ZSCAN` (`zscan`, `zscan_match`), `ZSCORE` (`zscore`) |
| **SERVER** | `PING`, `AUTH`, `DBSIZE`, `FLUSHDB`, `FLUSHALL`, `CONFIG`, `SAVE`, `SHUTDOWN`, `INFO`, `SELECT`, `TYPE`, `BGSAVE` |
| **TRANSACTION** | `MULTI`, `EXEC`, `DISCARD`, `WATCH` (`watch`), `UNWATCH` |
| **PUB/SUB** | `PUBLISH` (`publish`), `SUBSCRIBE` (`subscribe`), `UNSUBSCRIBE` (`unsubscribe_channels`), `PSUBSCRIBE` (`psubscribe`), `PUNSUBSCRIBE` (`punsubscribe_patterns`) |
| **GENERAL** | `KEYS` (`keys`), `SCAN`, `SORT` (`sort`, `sort_limit`, `sort_limit_order`), `TOUCH` (`touch`), `RENAME` (`rename`), `RENAMENX` (`renamemx`) |

</details>

---

## What works

- [`RedisClient`](./docs/architecture.md) — connect, execute commands, decode responses
- [`Pipeline`](./docs/architecture.md) — batch multiple commands in one round-trip
- [`InMemoryClient`](./docs/architecture.md) — test backend for unit and boundary tests
- **Pub/Sub client** — dedicated connection with push message parsing
- **96 Redis commands** — fully tested RESP2 encoding across 9 data categories
- **Redis Cluster** — CRC16 slot routing, MOVED/ASK redirect handling, topology discovery, multi-key fan-out
- **TLS/mTLS** — rustls 0.23, URL parsing (`rediss://`), server certs, client certs, SSRF protection
- **Docker test fixtures** — bollard-managed Redis containers (plain + TLS) via `--features test`
- **620 tests passing** — unit, integration, E2E, performance, doctests
- **SSRF protection** — blocks connections to private, link-local, loopback, reserved IPs
- **Command policy** — `AllowAll`, `DenyCommands`, `AllowCommands` with pre-build validation
- **JSF-AV compliant** — bounded complexity, no-panic dispatch, explicit types, lint profile

---

## What's next

- Connection pooling (reserved for a future epic)
- RESP3 type markers (`~`, `=`, `_`, `,`, `%`, `>`) — explicitly out of scope for v1
- Additional data types (Streams, Geo, HyperLogLog)

---

## Security

- **SSRF protection** — blocks connections to private, link-local, loopback,
  and reserved IP ranges after DNS resolution
- **Command policy** — `AllowAll`, `DenyCommands`, `AllowCommands`
  with O(1) `HashSet` lookups to block dangerous commands like
  `CONFIG`, `FLUSHALL`, `SHUTDOWN`

---

## Quick start

`may-redis` calls happen inside a `may` coroutine:

```rust
use may_redis::{RedisClient, Commands};

fn main() {
    may::run(|| {
        may::go(|| {
            let client = RedisClient::connect("127.0.0.1", 6379)
                .expect("Redis must be running on localhost:6379");

            // Inherent method that wraps PING and returns Result<String, RedisError>.
            assert_eq!(client.ping().unwrap(), "PONG");

            // Build a command with the Commands trait, run it with execute<T>.
            client.execute::<()>(client.set("greeting", "hello")).unwrap();
            let v: Option<String> =
                client.execute(client.get("greeting")).unwrap();
            assert_eq!(v.as_deref(), Some("hello"));

            // Pipeline several commands in one round-trip.
            let mut pipe = client.pipeline();
            pipe.add(client.set("a", "1"));
            pipe.add(client.set("b", "2"));
            pipe.add(client.get("a"));
            let ((), (), got_a): ((), (), Option<String>) =
                pipe.execute().unwrap();
            assert_eq!(got_a.as_deref(), Some("1"));
        })
        .join()
        .unwrap();
    });
}
```

A complete API tour — `Commands` trait method-by-method, `Pipeline`
tuple shapes, error handling, the runtime architecture diagram — lives
in [`docs/architecture.md`](./docs/architecture.md).

---

## Testing infrastructure

| Layer | Tooling | Commands | Tests | Notes |
|-------|---------|----------|-------|-------|
| Unit | `#[test]` | `cargo test --workspace` | 603 | No runtime, no network |
| Integration | `cargo test --features test` | `cargo test --features test` | 2 | Docker fixtures via bollard |
| E2E | `tests/fixture_e2e.rs` | `cargo test --test fixture_e2e --features test` | 2 | Docker plain + TLS containers |
| Perf | `tests/perf/` | `cargo test --test perf` | 7 | 2000-user JWT load scenarios |
| Doctests | `cargo test --doc` | `cargo test --doc` | 8 | Compile checks + CRC16 |

**Running all tests:** `cargo test --workspace --features test`

**Running unit tests only:** `cargo test --workspace`

**Test isolation:** Each integration test calls `FLUSHDB` before/after. Docker fixtures auto-create per-run Redis containers and clean up on drop.

**Concurrency tests:** Multiple coroutines sharing one client, pipeline ordering, backpressure — all verified under may's scheduler.

---

## Where to read next

| Audience | Start here |
|----------|------------|
| Architecture & runtime diagram | [`docs/architecture.md`](./docs/architecture.md) |
| All docs catalog | [`docs/INDEX.md`](./docs/INDEX.md) |
| Contributing | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |
| Test strategy | [`docs/10-test-strategy.md`](./docs/10-test-strategy.md) |
| Protocol analysis | [`docs/01-protocol-analysis.md`](./docs/01-protocol-analysis.md) |
| Knowledge base | [`llmwiki/index.md`](./llmwiki/index.md) |

---

## License

Dual-licensed under MIT or Apache-2.0, at your option. See
`Cargo.toml`.
