# may-redis

A coroutine-native Redis client built on the
[`may`](https://crates.io/crates/may) stackful-coroutine runtime.

**Zero tokio. Zero `async` / `.await`. Only `may` coroutines.**

---

## Why may-redis

Codebases that already run on `may` need Redis but don't want a second
runtime. `may-redis` eliminates that trade-off — same runtime, same
thread model, no `async` boilerplate, no `Pin`, no `BoxFuture`.

The public API surface deliberately mirrors the
[`redis`](https://crates.io/crates/redis) crate so migration is
mechanical: build a command, execute it, decode the typed result.

## Redis API coverage

`may-redis` implements the vast majority of the Redis command set with
correct RESP2 encoding tests. Every command is tested to produce the
exact wire format the Redis server expects.

| Category | Implemented | Tested | Coverage |
|----------|:-----------:|:------:|:--------:|
| STRING | 22 | 22 | 100% |
| HASH | 9 | 9 | 100% |
| SET | 11 | 11 | 100% |
| LIST | 12 | 12 | 100% |
| SORTED SET | 12 | 12 | 100% |
| SERVER | 12 | 12 | 100% |
| TRANSACTION | 4 | 4 | 100% |
| PUB/SUB | 5 | 5 | 100% |
| GENERAL | 8 | 8 | 100% |
| **Total** | **83** | **82** | **98.8%** |

> 1 command (`RENAMENX`) has a trait method but no encoding test — the sole
> gap.

### String commands

GET, SET, SETEX, SETNX, MGET, MSET, MSETNX, DEL, EXISTS, APPEND, STRLEN,
GETRANGE, SETRANGE, SETBIT, GETBIT, BITCOUNT, INCR, INCRBY, DECR, DECRBY,
TTL, PEXPIRE, PEXPIREAT, PERSIST, MOVE

### Hash commands

HSET, HGET, HMSET, HDEL, HGETALL, HKEYS, HLEN, HEXISTS, HINCRBY, HSCAN

### Set commands

SADD, SISMEMBER, SREM, SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION,
SMOVE, SSCAN

### List commands

LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP,
BRPOP

### Sorted set commands

ZADD, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZRANGE, ZRANGEBYSCORE,
ZRANK, ZREM, ZSCAN, ZSCORE

### Server / admin commands

PING, AUTH, DBSIZE, FLUSHDB, FLUSHALL, CONFIG, SAVE, SHUTDOWN, INFO, SELECT,
TYPE, BGSAVE

### Transaction commands

MULTI, EXEC, DISCARD, WATCH, UNWATCH

### Pub/Sub commands

PUBLISH, SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE

### General commands

KEYS, SCAN, SORT, TOUCH, RENAME, RENAMENX

## What works

- [`RedisClient`](./docs/architecture.md) — connect, execute commands, decode responses
- [`Pipeline`](./docs/architecture.md) — batch multiple commands in one round-trip
- [`InMemoryClient`](./docs/architecture.md) — test backend for unit and boundary tests
- **83 Redis commands** — fully tested RESP2 encoding across 7 data categories

## What's next

Pub/Sub full support (requires dedicated connection handling), MULTI/EXEC
transaction support, cluster/sentinel, TLS, and connection pooling are
planned for later epics. Out of scope for v1.

## Security

- **SSRF protection** — blocks connections to private, link-local,
  loopback, and reserved IP ranges after DNS resolution
- **Command policy** — `AllowAll`, `DenyCommands`, `AllowCommands`
  with O(1) `HashSet` lookups to block dangerous commands like
  `CONFIG`, `FLUSHALL`, `SHUTDOWN`

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

## Where to read next

| Audience | Start here |
|----------|------------|
| Architecture & runtime diagram | [`docs/architecture.md`](./docs/architecture.md) |
| All docs catalog | [`docs/INDEX.md`](./docs/INDEX.md) |
| Contributing | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |
| Full command coverage audit | [`docs/command-coverage-audit.md`](./docs/command-coverage-audit.md) |

## License

Dual-licensed under MIT or Apache-2.0, at your option. See
`Cargo.toml`.
