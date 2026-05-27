# may-redis

A coroutine-native Redis client for the `may` runtime.

Zero tokio. Zero async-await. Only may coroutines.

## Architecture

Single crate with modules:

- `base` — `RedisValue`, `RedisError`, `FromRedisValue`, `ToRedisArgs`
- `codec` — RESP encoding/decoding
- `protocol` — `CommandBuilder`, `Commands` trait
- `connection` — epoll connection loop, TCP, coroutine management
- `client` — `RedisClient`, `Pipeline`, public API

## Design Documents

Full system design in `docs/`:

1. [RESP Protocol Analysis](docs/01-protocol-analysis.md)
2. [may_postgres Comparison](docs/02-may_postgres_comparison.md)
3. [Sesame-IDAM Redis Usage](docs/03-sesame-idam-redis-usage.md)
4. [System Design](docs/04-system-design.md)
5. [Protocol Layer Design](docs/05-protocol-layer-design.md)
6. [Connection Layer Design](docs/06-connection-layer-design.md)
7. [Client API Design](docs/07-client-api-design.md)
8. [Module Structure](docs/08-module-structure.md)
9. [Migration Guide](docs/09-migration-guide.md)
10. [Test Strategy](docs/10-test-strategy.md)
11. [Dependencies](docs/11-dependencies.md)
