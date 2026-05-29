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

## Where to read next

For a full catalog of all docs, tests, architecture, and epics, see
[`docs/INDEX.md`](./docs/INDEX.md).

| Audience | Start here |
|----------|------------|
| Reading the codebase | [`docs/architecture.md`](./docs/architecture.md) |
| Contributing | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |
| Agent rule set (build/lint/test, modular targets, etc.) | [`AGENTS.md`](./AGENTS.md) |
| Implementing a specific feature | [`docs/Epics/`](./docs/Epics/) — each epic has `Story_0.md` (overview) + `Story_1..N.md` (tasks) |
| Understanding SSRF protection | [`llmwiki/concepts/ssrf-protection.md`](./llmwiki/concepts/ssrf-protection.md) |
| Understanding command policy | [`llmwiki/concepts/command-policy.md`](./llmwiki/concepts/command-policy.md) |
| Test strategy in depth | [`docs/10-test-strategy.md`](./docs/10-test-strategy.md) |

---

## License

Dual-licensed under MIT or Apache-2.0, at your option. See `Cargo.toml`.
