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

## What works

- [`RedisClient`](./docs/architecture.md) — connect, execute commands, decode responses
- [`Pipeline`](./docs/architecture.md) — batch multiple commands in one round-trip
- [`InMemoryClient`](./docs/architecture.md) — test backend for unit and boundary tests
- **30+ Redis commands** — `GET`, `SET`, `DEL`, `PING`, `EXISTS`, `TTL`, `EXPIRE`, `INCR`, `AUTH`, `PUBLISH`, `KEYS`, `DBSIZE`, `FLUSHDB`

## What's next

Pub/Sub, MULTI/EXEC, cluster/sentinel, TLS, and connection pooling are
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

## License

Dual-licensed under MIT or Apache-2.0, at your option. See
`Cargo.toml`.
