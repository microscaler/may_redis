# Performance Testing Implementation Plan

**Target:** `may-redis` — Redis performance validation for Sesame-IDAM access patterns at scale (2000 users).

**Purpose:** Validate that `may-redis` can handle the Redis load that Sesame-IDAM generates during normal operation, with realistic JWT-like key/value patterns and access frequency distributions.

---

## 1. JWT Structure & Claims

Each JWT-like token contains these claims (from sesame-idam `create_valid_jwt()`):

```json
{
    "sub": "user-{N}",
    "iss": "https://idam.example.com",
    "exp": <future_timestamp>,
    "iat": <now>,
    "nbf": <now>,
    "jti": "jti-{user_id}-{random}",
    "scope": "read write",
    "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "user_id": "user-{N}"
}
```

Redis stores JWT metadata under these key patterns (from `refresh_token.rs`):

| Key Pattern | Redis Type | TTL | Purpose |
|-------------|-----------|-----|---------|
| `refresh:{jti}` | Hash | 30 days | Refresh token metadata (jti, sub, sid, family_id, iat, exp, client_id, scopes) |
| `family:{family_id}` | Set | 24 hours | Token family members (for reuse detection) |
| `denylist:{jti}` | String | 24 hours | Revoked tokens (value: "rotated") |
| `session:{sid}` | Hash | 30 days | Session state (JSON blob) |

---

## 2. Redis Operations Per Flow

### Login Flow
1. `HSET refresh:{jti}` — store refresh token metadata (30d TTL)
2. `SADD family:{family_id} {jti}` — add to token family set
3. `SET session:{sid}` — store session state (30d TTL)
4. `HSET session:{sid}` — set session fields

### Token Refresh Flow
1. `HGET refresh:{jti}` — lookup old refresh token
2. `GET denylist:{jti}` — check if already revoked (4ms window)
3. `SISMEMBER family:{family_id} __REVOKED__` — check family revocation
4. `DEL refresh:{jti}` — invalidate old token
5. `SREM family:{family_id} {jti}` — remove from family
6. `SADD family:{family_id} __REVOKED__` — mark family revoked (if reuse detected)
7. `SET denylist:{jti}` — add old jti to denylist (24h TTL)
8. `HSET refresh:{new_jti}` — store new refresh token
9. `SADD family:{new_family} {new_jti}` — add to new family

### API Authorization Flow (EXTREME frequency)
1. JWT validation (in-memory, no Redis) — JWKS lookup + Ed25519 signature
2. `GET denylist:{jti}` — check if token revoked (local cache first, then Redis on miss)
3. `GET denylist:{jti}` — Redis fallback on cache miss (the 4ms window)

---

## 3. Access Frequency Distribution

Based on sesame-idam service topology:

| Service | Endpoint Type | Frequency | Redis Ops per Request |
|---------|--------------|-----------|----------------------|
| identity-login-service | Login/Register | HIGH | 2-3 ops |
| identity-session-service | Token refresh | HIGH | 5-9 ops |
| identity-user-mgmt-service | User CRUD | MEDIUM | 1-2 ops |
| **authz-core** | **API authorization** | **EXTREME** | **1-2 ops (denylist check)** |
| api-keys | M2M validation | HIGH | 1-2 ops |
| org-mgmt | Org lifecycle | LOW | 0-1 ops |

**Key insight:** authz-core is the EXTREME frequency service — every consumer API request hits it. The denylist check is the Redis bottleneck: 100% of requests, ~99% hit local cache, ~1% miss and hit Redis.

---

## 4. Performance Test Scenarios

### Scenario A: User Population (2000 users)

**Goal:** Populate Redis with realistic data for 2000 users.

**Operations per user:**
- 1 `refresh:{jti}` hash (~15 fields)
- 1 `family:{family_id}` set entry
- 1 `session:{sid}` hash (~5 fields)
- 0-1 `denylist:{jti}` entries (depends on rotation history)

**Total keys:** ~6000-8000 keys
**Total memory:** ~500KB-2MB (hashes are compact)

### Scenario B: Login Burst

**Goal:** Simulate N users logging in simultaneously.

**Pattern:** 2000 users, sequential login with N concurrent coroutines
**Per-user ops:** HSET refresh + SADD family + SET session + HSET session
**Metrics:** ops/sec, latency p50/p95/p99, memory growth

### Scenario C: Token Refresh Storm

**Goal:** Simulate token refresh across all 2000 users.

**Pattern:** Each user refreshes their token with the full 9-step flow
**Per-user ops:** HGET + GET denylist + SISMEMBER + DEL + SREM + SADD + SET denylist + HSET refresh + SADD family
**Metrics:** ops/sec, latency, Redis memory churn

### Scenario D: Authorization Load (EXTREME frequency)

**Goal:** Simulate authz-core denylist checks at scale.

**Pattern:** 10,000 requests/sec, 99% cache-hit simulation (skip Redis for most), 1% miss (actual Redis GET)
**Per-request ops:** 1 GET (denylist check)
**Metrics:** ops/sec, latency p50/p95/p99, cache hit ratio

### Scenario E: Mixed Workload (Realistic)

**Goal:** Simulate real Sesame-IDAM traffic mix.

**Distribution:**
- 50% authz-core denylist checks (GET denylist)
- 30% token refresh (HGET + GET + SISMEMBER + HSET + SADD)
- 15% login (HSET + SADD + SET + HSET)
- 5% user CRUD (HSET/HGET/HDEL)

**Metrics:** Overall throughput, latency distribution, memory usage

---

## 5. Implementation Approach

### Phase 1: Test Utilities (may-redis tests)

Create helper modules in `tests/perf/`:

```rust
// tests/perf/mod.rs
pub mod jwt;           // JWT claim generation
pub mod redis_ops;     // Redis operation helpers
pub mod metrics;       // Throughput/latency measurement
pub mod scenarios;     // Test scenario implementations
```

**JWT Claim Generator:**
```rust
// Generate 2000 unique JWT-like payloads
fn generate_user_jwt(user_id: u32) -> (String, RefreshTokenData) {
    let jti = format!("jti-{}-{}", user_id, random_hex(16));
    let sub = format!("user-{}", user_id);
    let sid = format!("sid-{}-{}", user_id, random_hex(8));
    let family_id = format!("fam-{}", user_id / 10);  // 10 users per family
    
    RefreshTokenData {
        jti, sub, sid, family_id,
        iat: now,
        exp: now + 3600,
        client_id: "web-app".to_string(),
        scopes: "read write".to_string(),
    }
}
```

**Redis Operation Helpers:**
```rust
// Populate Redis with 2000 users
fn populate_users(client: &RedisClient, count: usize) -> Result<Vec<RefreshTokenData>> {
    let mut users = Vec::new();
    for i in 0..count {
        let (jwt, data) = generate_user_jwt(i as u32);
        client.hset_many(&format!("refresh:{}", data.jti), &data.fields())?;
        client.sadd(&format!("family:{}", data.family_id), &data.jti)?;
        client.set_ex(&format!("session:{}", data.sid), &session_json, 30*86400)?;
        users.push(data);
    }
    Ok(users)
}
```

### Phase 2: Throughput Benchmarks

```rust
#[test]
fn bench_login_burst_2000_users() {
    let client = RedisClient::connect("redis://127.0.0.1:6379").unwrap();
    client.flushdb().unwrap();
    
    let start = Instant::now();
    let mut handles = Vec::new();
    
    // 100 coroutines, each handles 20 users
    for chunk in users.chunks(20) {
        let client = client.clone();
        let handle = may::go!(move || {
            for user in chunk {
                client.hset_many(&format!("refresh:{}", user.jti), &user.fields()).unwrap();
                client.sadd(&format!("family:{}", user.family_id), &user.jti).unwrap();
                client.set_ex(&format!("session:{}", user.sid), &user.session_json, 2592000).unwrap();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join();
    }
    
    let elapsed = start.elapsed();
    println!("2000 users logged in {} users/sec", 2000.0 / elapsed.as_secs_f64());
}
```

### Phase 3: Latency Distribution

```rust
#[test]
fn bench_authz_denylist_checks() {
    let client = RedisClient::connect("redis://127.0.0.1:6379").unwrap();
    // Pre-populate denylist entries for 2000 users
    for i in 0..2000 {
        client.set_ex(&format!("denylist:jti-{}", i), "rotated", 86400).unwrap();
    }
    
    let mut latencies = Vec::new();
    let mut handles = Vec::new();
    
    // 10,000 requests, 99% hit, 1% miss
    for i in 0..10000 {
        let client = client.clone();
        let handle = may::go!(move || {
            let start = Instant::now();
            if i % 100 == 0 {
                // Miss — key doesn't exist
                let _ = client.get(&format!("denylist:jti-missing-{}", i)).unwrap();
            } else {
                // Hit
                let _ = client.get(&format!("denylist:jti-{}", i % 2000)).unwrap();
            }
            latencies.push(start.elapsed());
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join();
    }
    
    latencies.sort();
    let p50 = latencies[latencies.len() / 20];
    let p95 = latencies[latencies.len() * 19 / 20];
    let p99 = latencies[latencies.len() * 99 / 100];
    
    println!("p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms", 
        p50.as_micros() as f64 / 1000.0,
        p95.as_micros() as f64 / 1000.0,
        p99.as_micros() as f64 / 1000.0);
}
```

---

## 6. Comparison Baseline

Test the same scenarios against:
1. **may-redis** (our implementation)
2. **redis crate** (official, tokio-based)

This gives a direct comparison of:
- Throughput (ops/sec)
- Latency (p50/p95/p99)
- Memory usage (RSS)
- Coroutine overhead vs tokio task overhead

---

## 7. CI Integration

Add to CI pipeline (after unit + integration tests):

```yaml
performance-tests:
  needs: integration-tests
  if: ${{ github.ref == 'refs/heads/main' }}  # Only on main branch
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run performance benchmarks
      run: cargo test --test perf --benchmarks
      env:
        REDIS_URL: redis://localhost:6379
    - name: Upload results
      uses: actions/upload-artifact@v4
      with:
        name: perf-results
        path: perf-results.json
```

**Note:** Performance tests are flaky (network, host load) — run only on main branch, not on PRs. Results are archival, not gating.

---

## 8. Output Format

Results written as JSON for CI artifact:

```json
{
    "scenario": "login_burst_2000",
    "users": 2000,
    "coroutines": 100,
    "total_ops": 6000,
    "throughput_ops_sec": 45000,
    "latency_ms": {
        "p50": 0.2,
        "p95": 0.8,
        "p99": 2.1
    },
    "memory_mb": 1.2,
    "timestamp": "2026-05-28T10:30:00Z"
}
```

---

## 9. File Structure

```
tests/perf/
├── mod.rs                  # Re-exports
├── jwt.rs                  # JWT claim generation
├── redis_ops.rs            # Redis operation helpers
├── metrics.rs              # Throughput/latency measurement
├── scenarios/
│   ├── mod.rs              # Re-exports
│   ├── population.rs       # Scenario A: 2000 user population
│   ├── login_burst.rs      # Scenario B: Login burst
│   ├── token_refresh.rs    # Scenario C: Token refresh storm
│   ├── authz_load.rs       # Scenario D: Authorization load
│   └── mixed_workload.rs   # Scenario E: Mixed workload
├── benchmark.rs            # Direct benchmark comparisons
└── README.md               # How to run, what metrics mean
```

---

## 10. Dependencies

Add to `Cargo.toml`:
```toml
[dev-dependencies]
# For performance testing
rand = "0.8"              # Random JTI generation
instant = "0.1"           # High-resolution timers
serde_json = "1.0"        # JSON serialization for sessions
```

No new runtime dependencies — everything uses existing `may` runtime primitives.

---

## 11. Success Criteria

| Metric | Target | Notes |
|--------|--------|-------|
| **2000-user population** | < 5 seconds | All 6000+ keys written |
| **Login burst (100 coroutines)** | < 3 seconds | 100 concurrent logins |
| **Authz denylist check** | < 1ms p99 | Redis GET latency |
| **Token refresh (50 coroutines)** | < 50 ops/sec/coroutine | 9-step flow |
| **Mixed workload (10K req)** | < 10 seconds | 50/30/15/5 distribution |
| **Memory growth** | < 5MB for 2000 users | Redis RSS |

These are soft targets — the goal is to identify bottlenecks, not pass/fail gates.

---

## 12. Implementation Order

1. **Phase 1:** JWT claim generator + Redis operation helpers (1 day)
2. **Phase 2:** Throughput benchmarks for login burst + authz load (1 day)
3. **Phase 3:** Latency distribution + mixed workload (1 day)
4. **Phase 4:** CI integration + documentation (0.5 day)

Total: ~3.5 days of implementation.
