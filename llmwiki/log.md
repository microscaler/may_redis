# Wiki Log

> Chronological record of all wiki actions. Append-only.
> Format: `## [YYYY-MM-DD] action | subject`

## [2026-06-01] create | Wiki initialized
- Domain: Project infrastructure, architecture, and engineering decisions
- Structure created with SCHEMA.md, index.md, log.md

## [2026-06-01] update | Redis command coverage audit
- Created comparison page: redis-command-coverage.md
- Documents 20/82+ commands implemented (~24% coverage)
- Confirms 100% sesame-idam command coverage (all 11 canonical commands implemented)
- Lists 60+ missing commands by category (HASH, SET, LIST, SORTED SET, STRING EXTENSION, PUB/SUB, TRANSACTIONS, SERVER/ADMIN)

## [2026-06-01] create | Redis command expansion epic
- Created Epic 7 stories for systematic Redis command expansion
- Story 1: String Extension (DECR, SETNX, MGET, MSET, STRLEN, GETRANGE, SETBIT, GETBIT, BITCOUNT)
- Story 2: Hash (HDEL, HKEYS, HGETALL, HMSET, HINCRBY, HLEN, HEXISTS, HSCAN)
- Story 3: Set (SMEMBERS, SPOP, SRANDMEMBER, SCARD, SINTER, SUNION, SMOVE, SSCAN)
- Story 4: List (LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX, LSET, LREM, LTRIM, BLPOP, BRPOP)
- Story 5: Sorted Set (ZADD, ZREM, ZRANGE, ZRANK, ZSCORE, ZCARD, ZCOUNT, ZINCRBY, ZPOPMAX, ZPOPMIN, ZSCAN, ZRANGEBYSCORE)
- Story 6: Pub/Sub + Transactions (SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE, MULTI, EXEC, DISCARD, WATCH, UNWATCH)
- Story 7: Server/Admin (SELECT, TYPE, MOVE, RENAME, RENAMENX, SORT, SCAN, TOUCH, SAVE, BGSAVE, FLUSHALL, PTTL, PEXPIRE, PERSIST, SHUTDOWN, INFO, CONFIG)
