# may-redis

[![CI](https://github.com/microscaler/may_redis/actions/workflows/ci.yaml/badge.svg)](https://github.com/microscaler/may_redis/actions/workflows/ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-Apache)

A coroutine-native Redis client built on the
[`may`](https://crates.io/crates/may) stackful-coroutine runtime.

**Zero tokio. Zero `async` / `.await`. Only `may` coroutines.**

This crate exists so codebases that already run on `may` can talk to
Redis without dragging a second runtime into the process. The public
API surface deliberately mirrors the [`redis`](https://crates.io/crates/redis)
crate so migration is mechanical: build a command, execute it, decode
the typed result.

> **Status: pre-1.0, internal use only.** Single-connection client,
> `Pipeline`, and `InMemoryClient` (test backend) work end-to-end against a
> real Redis. Pub/Sub, MULTI/EXEC, cluster/sentinel, TLS, and connection
> pooling are explicitly out of scope for v1 — see
> [`docs/architecture.md`](./docs/architecture.md) section 1.

---

## Quick start

`may-redis` calls must happen inside a `may` coroutine. From a
`main`, that means wrapping in `may::run(..)` and spawning a coroutine
with `may::go(..)`:

```rust
use may_redis::{RedisClient, Commands};

fn main() {
    may::run(|| {
        may::go(|| {
            // Host + port, NOT a single "host:port" string. Use
            // RedisClient::connect_url("redis://host:port") for the URL form.
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

## Building and testing

```bash
cargo build                                            # build the crate
cargo build --release                                  # release build

cargo test                                             # unit tests only
cargo test --features test -- --test-threads=1         # unit + integration
                                                       # (needs Redis on
                                                       #  127.0.0.1:6379)

cargo fmt                                              # format
cargo clippy --lib --tests --all-features              # lint
cargo doc --no-deps --open                             # browse rustdoc
```

The integration tests (`client::client::tests::test_integration_*`) require
a Redis server on `127.0.0.1:6379` and **must** run with
`--test-threads=1` — they share Redis state and `FLUSHDB` between
assertions. See [`docs/architecture.md`](./docs/architecture.md) section 9
for the full testing-architecture rules.

---

## Project conventions (the short version)

The full rule set lives in [`AGENTS.md`](./AGENTS.md). The non-negotiables
are:

- **Only the `may` runtime.** No `tokio`, no `async-std`, no `smol`. No
  `async fn`, no `.await`, no `#[tokio::test]`. The connection layer
  uses `may::go!`, `may::net::TcpStream`, `may::sync::spsc`,
  `may::queue::mpsc::Queue`, and `WaitIo` / `WaitIoWaker`.
- **RESP2 only for v1.** RESP3 type markers are explicitly out of
  scope.
- **Reference implementation:** `../may_postgres/src/connection.rs`.
  Any change to `src/connection/connection.rs::spawn_connection_loop`
  must be justified against that loop. The connection loop has shipped
  two production-impacting bugs that caused integration tests to hang —
  both are dissected in
  [`llmwiki/topics/connection-loop-pitfalls.md`](./llmwiki/topics/connection-loop-pitfalls.md)
  with regression tests. Read it before touching the loop.
- **Conventional Commits.** `feat(scope): …`, `fix(scope): …`,
  `docs(scope): …`, `chore(scope): …`, `refactor(scope): …`.
- **Never push without explicit human authorization.** Never use
  `--no-verify`. Never commit secrets.

---

## Where to read next

| Audience | Start here |
|----------|------------|
| Reading the codebase | [`docs/architecture.md`](./docs/architecture.md) |
| Working on the crate as an agent | [`AGENTS.md`](./AGENTS.md) |
| Implementing a specific feature | [`docs/Epics/`](./docs/Epics/) — Epic 0 → Epic 6, each with `Story_0.md` (overview) + `Story_1..N.md` (granular stories) |
| Touching the connection loop | [`llmwiki/topics/connection-loop-pitfalls.md`](./llmwiki/topics/connection-loop-pitfalls.md) |
| Understanding the RESP wire format | [`docs/01-protocol-analysis.md`](./docs/01-protocol-analysis.md) |
| Understanding may-postgres patterns we mirror | [`docs/02-may_postgres_comparison.md`](./docs/02-may_postgres_comparison.md) |
| Cataloguing Redis usage in downstream code | [`docs/03-sesame-idam-redis-usage.md`](./docs/03-sesame-idam-redis-usage.md) |
| Test strategy in depth | [`docs/10-test-strategy.md`](./docs/10-test-strategy.md) |
| Why a single crate, not a workspace | [`docs/adr-001-single-crate-structure.md`](./docs/adr-001-single-crate-structure.md) |

---

## License

Dual-licensed under MIT or Apache-2.0, at your option. See `Cargo.toml`.
