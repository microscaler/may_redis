# may-redis — Contributing

> Project conventions, coding standards, and how to contribute to may-redis.

## Only the `may` runtime

No `tokio`, no `async-std`, no `smol`. No `async fn`, no `.await`, no `#[tokio::test]`. The connection layer uses `may::go!`, `may::net::TcpStream`, `may::sync::spsc`, `may::queue::mpsc::Queue`, and `WaitIo` / `WaitIoWaker`.

## RESP2 only for v1

RESP3 type markers are explicitly out of scope.

## Reference implementation

`../may_postgres/src/connection.rs`. Any change to `src/connection/connection.rs::spawn_connection_loop` must be justified against that loop. The connection loop has shipped two production-impacting bugs that caused integration tests to hang — both are dissected in [`llmwiki/topics/connection-loop-pitfalls.md`](./llmwiki/topics/connection-loop-pitfalls.md) with regression tests. **Read it before touching the loop.**

## Commit conventions

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(scope): …`, `fix(scope): …`, `docs(scope): …`, `chore(scope): …`, `refactor(scope): …`.

**Never push without explicit human authorization.** Never use `--no-verify`. Never commit secrets.

## Full rule set

The complete agent rule set (build/lint/test commands, modular architecture targets, commit discipline, etc.) lives in [`AGENTS.md`](./AGENTS.md). Use that as your source of truth when working on this codebase.
