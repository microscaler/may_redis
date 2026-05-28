# Wiki Schema

## Domain
Project infrastructure, architecture, and engineering decisions — may-redis, sesame-idam, BRRTRouter, and related microservices ecosystem.

## Conventions
- File names: lowercase, hyphens, no spaces (e.g., `may-redis-architecture.md`)
- Every wiki page starts with YAML frontmatter (see below)
- Use `[[wikilinks]]` to link between pages (minimum 2 outbound links per page)
- When updating a page, always bump the `updated` date
- Every new page must be added to `index.md` under the correct section
- Every action must be appended to `log.md`

## Frontmatter
```yaml
---
title: Page Title
created: YYYY-MM-DD
updated: YYYY-MM-DD
type: entity | concept | comparison | query | summary
tags: [from taxonomy below]
sources: [raw/articles/source-name.md]
---
```

## Tag Taxonomy
- Project: project, microservice, library, client, server
- Architecture: architecture, redis, jwt, auth, identity, coroutine, epoll, pipeline
- Database: redis, postgres, cache, session, token
- Operations: testing, perf, ci, lint, coverage
- Security: jwt, dpop, token-versioning, denylist, refresh-token
- Protocol: resp, redis-protocol, wire-format

Rule: every tag on a page must appear in this taxonomy. If a new tag is needed, add it here first, then use it.

## Page Thresholds
- Create a page when an entity/concept appears in 2+ sources OR is central to one source
- Add to existing page when a source mentions something already covered
- Don't create a page for passing mentions, minor details, or things outside the domain
- Split a page when it exceeds ~200 lines
- Archive a page when its content is fully superseded — move to `_archive/`, remove from index

## Entity Pages
One page per notable entity. Include: overview, key facts, relationships to other entities (wikilinks), source references.

## Concept Pages
One page per concept or topic. Include: definition, current state of knowledge, open questions, related concepts (wikilinks).

## Comparison Pages
Side-by-side analyses. Include: what is being compared and why, dimensions of comparison (table), verdict, sources.

## Update Policy
When new information conflicts with existing content:
1. Check dates — newer sources generally supersede older ones
2. If genuinely contradictory, note both positions with dates and sources
3. Mark the contradiction in frontmatter: `contradictions: [page-name]`
4. Flag for user review in the lint report
