# LLM Wiki Log

## [2026-05-27] ingest | bootstrap llmwiki from docs

- Created initial `llmwiki/` structure with SCHEMA.md, index.md, docs-catalog.md, log.md.
- Imported full `docs/**/*.md` inventory (11 files) into docs-catalog.md.
- Created topic stubs for RESP protocol, may coroutine pattern, sesame-IDAM integration, module structure.
- Created reference stubs for codebase entry points and command mapping.
- All pages marked `unverified` — docs are design/planning artifacts; code is a single crate with flat modules, not yet the planned modular workspace.
- Created `/home/casibbald/Workspace/microscaler/may_redis/AGENTS.md` referencing the wiki.

## [2026-05-27] ingest | decompose design docs into 7 epics with granular stories

- Created `docs/Epics/` directory structure with 7 epics:
  - `00-epic-overview.md` — project goal, architecture diagrams, execution rules
  - `epic-0-scaffolding.md` — 4 stories (workspace, modules, lint, docs)
  - `epic-1-base.md` — 4 stories (RedisValue, RedisError, ToRedisArgs, full FromRedisValue)
  - `epic-2-codec.md` — 3 stories (RESPWriter, RESPReader, full RESP2 + roundtrip)
  - `epic-3-protocol.md` — 4 stories (CommandBuilder, Commands trait, Request/Response, integration)
  - `epic-4-connection.md` — 4 stories (TcpConnector, Connection struct, epoll loop, integration)
  - `epic-5-client.md` — 3 stories (RedisClient, Pipeline, InMemoryClient)
  - `epic-6-integration.md` — 4 stories (workspace test pass, concurrency, error handling, migration guide)
- Moved source design docs into epic directories:
  - `docs/04-07` → `docs/Epics/epic-3-protocol/docs/` (protocol/client design)
  - `docs/08-11` → `docs/Epics/epic-0-scaffolding/docs/` (module structure, migration, dependencies)
- Each story includes: code anchors, mermaid diagrams (graph/flowchart/sequence), structured tasks, verification criteria
- Updated AGENTS.md to reference epics instead of raw docs
- Total: 26 granular stories across 7 epics, all independently verifiable
