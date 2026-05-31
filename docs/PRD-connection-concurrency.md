# PRD: Redis Connection Concurrency Model

## Overview

This document analyzes and documents the correct concurrency model for
`may-redis`: whether to use a single multiplexed connection or multiple
connections when multiple may coroutines call Redis concurrently.

**Status:** Draft — awaiting review
**Author:** [agent]
**Date:** 2025-01-15

## 1. Problem Statement

`may-redis` is built on the may coroutine runtime. Multiple application
coroutines may simultaneously call `RedisClient.execute()`. The question:
how should these coroutines reach the Redis server?

- **Approach A: Multiple connections** — each coroutine (or each service pod)
  opens its own TCP connection.
- **Approach B: Single multiplexed connection** — all coroutines share one
  TCP connection. Requests are queued in an mpsc `Queue<Request>` and
  dispatched by a single epoll loop.

The current implementation uses Approach B (single shared connection).
This PRD documents why that is correct and proposes optional additions
for resilience.

## 2. Key Facts

### 2.1 Redis is Single-Threaded

Redis processes commands **one at a time**, regardless of how many TCP
connections it has. A connection count of 1 or 100 makes zero difference
to throughput. Redis is the bottleneck, not the connection layer.

Typical throughput: 100K–500K ops/sec depending on command type.

### 2.2 May Coroutines Are Cooperative

Spawning 10 coroutines does **not** spawn 10 OS threads. All 10 share
one may worker. They yield cooperatively. Yielding on I/O (via
`spsc::Receiver::recv()`) suspends that coroutine and lets another run.

### 2.3 The Current Architecture (Already Muxed)

```
Coroutine A ──► RedisClient.execute() ──► req_queue.push()
Coroutine B ──► RedisClient.execute() ──► req_queue.push()
Coroutine C ──► RedisClient.execute() ──► req_queue.push()
                                         │
                                         ▼
                                   ┌──────────────┐
                                   │  Epoll Loop   │
                                   │  go! coroutine │────► TCP Socket ──► Redis
                                   └──────────────┘
                                         │
                                         ├──► spsc chan → A wakes on recv()
                                         ├──► spsc chan → B wakes on recv()
                                         └──► spsc chan → C wakes on recv()
```

`RedisClient` is `Arc<InnerClient>`. Every clone shares the same
`Connection`, which owns:
- One TCP socket
- One epoll loop (`go!` coroutine)
- One mpsc `Queue<Request>` (lock-free)
- One `WaitIoWaker`

The connection loop drains the queue, writes commands to the socket,
reads responses, and dispatches each response to the correct coroutine
via a per-request `spsc::Sender`.

### 2.4 The Epoll Loop Is Not a Bottleneck

- It uses non-blocking I/O with `epoll_wait` — never blocks, never busy-spins
- Drains the entire `req_queue` each iteration (O(1) lock-free pop)
- Writes multiple pipelined commands per TCP round-trip
- Reads and decodes multiple responses per TCP read (RESP batching)
- Is a single `go!` coroutine that runs continuously between yields

### 2.5 Current Backpressure

`Connection` has a configurable `max_queue_depth` (default: 1024
pending requests). If exceeded, `send()` returns
`ConnectionLimitError::QueueFull`. This prevents unbounded memory growth
under load.

## 3. Comparison: Single vs. Multiple Connections

| Aspect | Single Connection | N Connections |
|--------|------------------|---------------|
| Redis throughput | N ops/sec (bottlenecked by Redis) | N ops/sec (bottlenecked by Redis — same) |
| TCP sockets | 1 | N |
| Memory | 1 × buffer pool | N × buffer pools |
| File descriptors | 1 | N |
| Epoll loop coroutines | 1 | N |
| Queue contention | Shared queue (lock-free mpsc) | No contention, but N× overhead |
| Fault isolation | 1 failure kills all requests | 1 failure affects fraction |
| HOL blocking | Yes (slow cmd blocks all) | Partial (slow cmd blocks its connection only) |
| Pipeline efficiency | High (commands batched on wire) | Reduced (N separate round-trips) |

## 4. Analysis

### 4.1 Why Multiple Connections Is Wrong (For Most Workloads)

The ONLY real benefits of multiple connections to a single Redis
instance are:
1. **Fault isolation** — one dropped connection does not take down the others
2. **Partial HOL avoidance** — a slow command on connection 1 does not
   block connection 2

Both are **defensive** (worst-case) benefits. They add N× resource
overhead for potentially no measurable benefit.

Redis throughput is unchanged. File descriptors increase linearly.
Memory increases linearly. Epoll loop count increases linearly.
Pipeline efficiency decreases (separate round-trips per connection).

### 4.2 The Correct Default: Single Multiplexed Connection

This is already what the library does. Each service pod creates one
`RedisClient`, shares it via `Arc` across all coroutines. This is the
correct default for:
- Single Redis instance deployments
- Redis Sentinel (failover at the application level)
- Maximum throughput (pipelining, zero connection overhead)
- Minimal resource usage (1 socket, 1 epoll loop)

### 4.3 The Correct Enhancement: Optional Connection Pool

For fault resilience, add an optional `RedisConnectionPool`:

```
Service Pod
┌──────────────────────────────────────┐
│                                      │
│  Coroutine A ─┐                      │
│  Coroutine B ─┼──► ConnectionPool   │
│  Coroutine C ─┘     │               │
│                    ┌──┴──┐           │
│                    │Pool  │           │
│              ┌─────┤ of N │─────► Redis
│              │     │Conn  │           │
│  Round-robin │     │ Conn │           │
│  selection   │     └──────┘           │
└──────────────┘                        │
                                        │
                          (Automatic reconnect on failure)
```

- Pool size: small (2–4 connections)
- Selection: round-robin or hash-by-key
- Reconnect: automatic on connection failure
- Benefits: fault tolerance without N× overhead
- This is useful for Redis Cluster (sharding) AND single-instance resilience

## 5. Recommended Architecture

### 5.1 Default: Single Multiplexed Connection (No Change)

```
Service Pod
┌──────────────────────────────────┐
│                                  │
│  Coroutine 1 ─┐                  │
│  Coroutine 2 ─┼── Arc<Client>   │
│  Coroutine N ─┘                  │
│                                  │
│  (1 TCP socket)                  │
│  (1 epoll loop)                  │
│                                  │
└──────────────────────────────────┘
              │
              ▼
     ┌─────────────────┐
     │  Redis Server    │
     │  (1 connection)  │
     └─────────────────┘
```

**What this gives:**
- Best performance: 1 socket, 1 epoll loop, lock-free queue
- Maximum throughput (pipelining, zero connection overhead)
- Minimal resource usage
- Backpressure via `max_queue_depth`
- Already implemented and production-tested

### 5.2 Optional: Connection Pool Layer (New)

```
Service Pod
┌──────────────────────────────────────────────┐
│                                              │
│  Coroutine A ─┐                              │
│  Coroutine B ─┼──► ConnectionPool (N conn)   │
│  Coroutine C ─┘     │                        │
│                    ┌─┴──┐                     │
│                    │Pool │                   Redis
│              ┌─────┤ of N│──┐                │
│              │     │Conn │  │                │
│  Round-robin │     │ Conn│  │                │
│  selection   │     └─────┘  │                │
│              │              │                │
│  ┌───────────┴──────────────┴──────────────┐│
│  │  Connection Pool (optional, 2–4 conns)   ││
│  │  - Round-robin or hash-by-key selection  ││
│  │  - Automatic reconnect on failure        ││
│  │  - Per-connection backpressure           ││
│  └─────────────────────────────────────────┘│
└──────────────────────────────────────────────┘
```

**What this adds:**
- Fault tolerance: one connection drop does not take down all requests
- Partial HOL avoidance: slow command on one connection does not block others
- Redis Cluster support: connections to different shards

**What this does NOT add:**
- Throughput improvement (Redis is still single-threaded)
- Better pipeline efficiency (each connection is its own pipeline)

## 6. Implementation Plan

### Phase 1: No Change (Current State)

The default `RedisClient` uses a single multiplexed connection.
No code changes needed. Document this as the recommended pattern.

### Phase 2: Optional Connection Pool (Future)

1. Create `RedisConnectionPool` struct in `src/client/pool.rs`
2. Pool manages N `Arc<InnerClient>` connections
3. Round-robin or hash-based connection selection
4. Automatic reconnect on connection failure (per-connection)
5. Pool-level backpressure (aggregate of all connections)
6. Optional API: `RedisClient::with_pool(host, port, size)` vs `RedisClient::connect(host, port)`

## 7. Decision

**Default: single multiplexed connection (existing behavior).**

This is the correct default for all workloads where Redis is the
throughput bottleneck (which is everything). Multiple connections
add N× overhead for ZERO throughput gain.

Connection pool is an optional layer for resilience, added when
fault tolerance matters more than maximum throughput.

## 8. References

- [`docs/02-may_postgres_comparison.md`](02-may_postgres_comparison.md) — reference architecture
- [`docs/Epics/Epic_0/Story_0.md`](Epics/Epic_0/Story_0.md) — module structure and dependency graph
- [`src/connection/connection.rs`](../src/connection/connection.rs) — current implementation
- [`src/connection/epoll_loop.rs`](../src/connection/epoll_loop.rs) — epoll loop design
- [`src/connection/dispatch.rs`](../src/connection/dispatch.rs) — request/response dispatch
- [`src/connection/connection_limits.rs`](../src/connection/connection_limits.rs) — backpressure limits
- [`src/client/client.rs`](../src/client/client.rs) — RedisClient API
- [`src/client/pipeline.rs`](../src/client/pipeline.rs) — pipeline batching
