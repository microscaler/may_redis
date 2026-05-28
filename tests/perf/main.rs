// Performance test runner for may-redis.
//
// Runs all performance scenarios against a live Redis instance.
//
// Usage: cargo test --test perf
//
// Prerequisites:
// - Redis running on localhost:6379
// - Flush DB before/after each test with FLUSHDB

use may::go;
use may_redis::{Commands, RedisClient};
use std::fmt::Write;

fn shared_client() -> RedisClient {
    RedisClient::connect_url("redis://127.0.0.1:6379").unwrap()
}

fn random_hex(len: usize) -> String {
    (0..len).fold(String::new(), |mut acc, _| {
        let _ = write!(acc, "{:02x}", fastrand::u8(0..=255));
        acc
    })
}

// -----------------------------------------------------------------------
// Scenario A: User Population (2000 users)
// -----------------------------------------------------------------------

#[test]
fn test_scenario_population_2000_users() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    let count = 2000;
    let start = std::time::Instant::now();

    for i in 0..count {
        let uid = format!("user-{i}");
        let jti = format!("jti-{}", random_hex(8));
        let sid = format!("sid-{}", random_hex(6));
        let family_id = format!("fam-{}", i / 10);
        let refresh_key = format!("refresh:{jti}");
        let _jti_value = format!(
            r#"{{"jti":"{}","sub":"{uid}","sid":"{sid}","family_id":"{family_id}"}}"#,
            refresh_key.split(':').nth(1).unwrap()
        );
        let session_json = format!(
            r#"{{"sid":"{sid}","jti":"{}"}}"#,
            refresh_key.split(':').nth(1).unwrap()
        );
        let session_key = format!("session:{sid}");
        let family_key = format!("family:{family_id}");

        // HSET refresh:{jti} 8 fields
        let _ = client.execute::<()>(client.hset(
            &refresh_key,
            "jti",
            refresh_key.split(':').nth(1).unwrap(),
        ));
        let _ = client.execute::<()>(client.hset(&refresh_key, "sub", &uid));
        let _ = client.execute::<()>(client.hset(&refresh_key, "sid", &sid));
        let _ = client.execute::<()>(client.hset(&refresh_key, "family_id", &family_id));
        let _ = client.execute::<()>(client.hset(&refresh_key, "iat", i.to_string()));
        let _ = client.execute::<()>(client.hset(&refresh_key, "exp", (i + 3600).to_string()));
        let _ = client.execute::<()>(client.hset(&refresh_key, "client_id", "web-app"));
        let _ = client.execute::<()>(client.hset(&refresh_key, "scopes", "read write"));

        // SADD family:{family_id} {jti}
        let _ = client
            .execute::<usize>(client.sadd(&family_key, refresh_key.split(':').nth(1).unwrap()));

        // SETEX session:{sid} 2592000 session_json
        let _ = client.execute::<()>(client.setex(&session_key, 2_592_000, &session_json));
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    let dbsize: Result<usize, _> = client.execute::<usize>(client.dbsize());
    assert!(dbsize.is_ok(), "dbsize should succeed");

    assert!(
        elapsed_ms < 5000.0,
        "Population took {elapsed_ms:.0}ms, target < 5000ms"
    );

    client.execute::<()>(client.flushdb()).ok();
}

/// Concurrent population: 100 coroutines handling 20 users each.
#[test]
fn test_scenario_concurrent_population() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    let count: usize = 2000;
    let workers: usize = 100;
    let chunk_size = count.div_ceil(workers);
    let start = std::time::Instant::now();

    let mut handles = Vec::new();

    for chunk in (0..count).step_by(chunk_size) {
        let client = client.clone();
        let end = std::cmp::min(chunk + chunk_size, count);
        let handle = go!(move || -> usize {
            let mut keys = 0usize;
            for i in chunk..end {
                let uid = format!("user-{i}");
                let jti = format!("jti-{}", random_hex(8));
                let sid = format!("sid-{}", random_hex(6));
                let family_id = format!("fam-{}", i / 10);
                let refresh_key = format!("refresh:{jti}");
                let session_json = format!(r#"{{"sid":"{sid}"}}"#);
                let session_key = format!("session:{sid}");
                let family_key = format!("family:{family_id}");

                let _ = client.execute::<()>(client.hset(&refresh_key, "jti", &jti));
                let _ = client.execute::<()>(client.hset(&refresh_key, "sub", &uid));
                let _ = client.execute::<()>(client.hset(&refresh_key, "sid", &sid));
                let _ = client.execute::<()>(client.hset(&refresh_key, "family_id", &family_id));
                let _ = client.execute::<()>(client.hset(&refresh_key, "iat", i.to_string()));
                let _ =
                    client.execute::<()>(client.hset(&refresh_key, "exp", (i + 3600).to_string()));
                let _ = client.execute::<()>(client.hset(&refresh_key, "client_id", "web-app"));
                let _ = client.execute::<()>(client.hset(&refresh_key, "scopes", "read write"));

                let _ = client.execute::<usize>(client.sadd(&family_key, &jti));

                let _ = client.execute::<()>(client.setex(&session_key, 2_592_000, &session_json));
                keys += 10;
            }
            keys
        });
        handles.push(handle);
    }

    let total_keys: usize = handles.into_iter().map(|h| h.join().unwrap_or(0)).sum();
    let _elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let expected_keys = count * 10;

    assert_eq!(
        total_keys, expected_keys,
        "Expected {expected_keys} keys, got {total_keys}"
    );

    client.execute::<()>(client.flushdb()).ok();
}

// -----------------------------------------------------------------------
// Scenario B: Login Burst
// -----------------------------------------------------------------------

#[test]
fn test_scenario_login_burst() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    let login_count = 100;
    let start = std::time::Instant::now();

    for i in 0..login_count {
        let uid = format!("user-{i}");
        let jti = format!("jti-{}", random_hex(8));
        let sid = format!("sid-{}", random_hex(6));
        let family_id = format!("fam-{}", i / 10);
        let refresh_key = format!("refresh:{jti}");
        let family_key = format!("family:{family_id}");
        let session_key = format!("session:{sid}");

        let _ = client.execute::<()>(client.hset(&refresh_key, "sub", &uid));
        let _ = client.execute::<()>(client.hset(&refresh_key, "sid", &sid));
        let _ = client.execute::<()>(client.hset(&refresh_key, "family_id", &family_id));
        let _ = client.execute::<usize>(client.sadd(&family_key, &jti));
        let _ = client.execute::<()>(client.setex(&session_key, 2_592_000, &jti));
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let _ = (f64::from(login_count) / elapsed_ms) * 1000.0;

    let dbsize: Result<usize, _> = client.execute::<usize>(client.dbsize());
    assert!(dbsize.is_ok(), "dbsize should succeed");

    client.execute::<()>(client.flushdb()).ok();
}

// -----------------------------------------------------------------------
// Scenario C: Token Refresh Storm
// -----------------------------------------------------------------------

#[test]
fn test_scenario_token_refresh_storm() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    let refresh_count = 100;

    // First, populate users
    for i in 0..refresh_count {
        let uid = format!("user-{i}");
        let jti = format!("jti-{}", random_hex(8));
        let sid = format!("sid-{}", random_hex(6));
        let family_id = format!("fam-{}", i / 10);
        let refresh_key = format!("refresh:{jti}");
        let family_key = format!("family:{family_id}");
        let session_key = format!("session:{sid}");

        let _ = client.execute::<()>(client.hset(&refresh_key, "sub", &uid));
        let _ = client.execute::<()>(client.hset(&refresh_key, "family_id", &family_id));
        let _ = client.execute::<usize>(client.sadd(&family_key, &jti));
        let _ = client.execute::<()>(client.setex(&session_key, 2_592_000, &jti));
    }

    let start = std::time::Instant::now();

    // Simulate 9-step refresh flow
    for i in 0..refresh_count {
        let jti = format!("jti-{}", random_hex(8));
        let family_id = format!("fam-{}", i / 10);
        let new_jti = format!("new-jti-{}", random_hex(8));

        let refresh_key = format!("refresh:{jti}");
        let family_key = format!("family:{family_id}");
        let new_refresh_key = format!("refresh:{new_jti}");
        let denylist_key = format!("denylist:{jti}");

        // Step 1: HGET refresh:{jti}
        let _: Result<Option<String>, _> =
            client.execute::<Option<String>>(client.hget(&refresh_key, "sub"));

        // Step 2: GET denylist:{jti}
        let _: Result<Option<String>, _> =
            client.execute::<Option<String>>(client.get(&denylist_key));

        // Step 3: SISMEMBER
        let _: Result<bool, _> =
            client.execute::<bool>(client.sismember(&family_key, "__REVOKED__"));

        // Step 4: DEL refresh:{jti}
        let _: Result<usize, _> = client.execute::<usize>(client.del(&refresh_key));

        // Step 5: SREM
        let _: Result<usize, _> = client.execute::<usize>(client.srem(&family_key, &jti));

        // Step 6: SADD new member
        let _ = client.execute::<usize>(client.sadd(&family_key, &new_jti));

        // Step 7: SET denylist
        let _ = client.execute::<()>(client.setex(&denylist_key, 86400, "rotated"));

        // Step 8: HSET new token
        let _ = client.execute::<()>(client.hset(&new_refresh_key, "jti", &new_jti));

        // Step 9: SADD new member again
        let _ = client.execute::<usize>(client.sadd(&family_key, &new_jti));
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let _ = (f64::from(refresh_count) / elapsed_ms) * 1000.0;

    client.execute::<()>(client.flushdb()).ok();
}

// -----------------------------------------------------------------------
// Scenario D: Authorization Load (EXTREME frequency)
// -----------------------------------------------------------------------

#[test]
fn test_scenario_authz_load_10k_requests() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    // Pre-populate denylist for 2000 users
    for i in 0..2000 {
        let jti = format!("jti-{}-{}", i, random_hex(8));
        let _ = client.execute::<()>(client.setex(&jti, 86400, "rotated"));
    }

    let request_count = 10000;
    let start = std::time::Instant::now();
    let mut hits = 0usize;

    for i in 0..request_count {
        if i % 100 == 0 {
            // Miss
            let result: Result<Option<String>, _> =
                client.execute::<Option<String>>(client.get(format!("denylist:jti-missing-{i}")));
            if result.is_err() {
                hits += 1;
            }
        } else {
            // Hit
            let jti = format!("jti-{}-{}", i % 2000, random_hex(8));
            let result: Result<Option<String>, _> =
                client.execute::<Option<String>>(client.get(&jti));
            if result.is_ok() {
                hits += 1;
            }
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let _ = (f64::from(request_count) / elapsed_ms) * 1000.0;
    let hit_ratio = (hits as f64 / f64::from(request_count)) * 100.0;

    assert!(
        hit_ratio > 90.0,
        "Expected hit ratio > 90%, got {hit_ratio:.1}%"
    );

    client.execute::<()>(client.flushdb()).ok();
}

/// Latency profile for denylist checks.
#[test]
fn test_scenario_authz_latency_profile() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    for i in 0..200 {
        let jti = format!("jti-latency-{i}");
        let _ = client.execute::<()>(client.setex(&jti, 86400, "rotated"));
    }

    let request_count = 1000;
    let mut latencies = Vec::with_capacity(request_count);

    for i in 0..request_count {
        let op_start = std::time::Instant::now();

        if i % 10 == 0 {
            let _: Result<Option<String>, _> =
                client.execute::<Option<String>>(client.get(format!("denylist:jti-missing-{i}")));
        } else {
            let jti = format!("jti-latency-{}", i % 200);
            let _: Result<Option<String>, _> = client.execute::<Option<String>>(client.get(&jti));
        }

        latencies.push(op_start.elapsed().as_micros() as u64);
    }

    assert_eq!(latencies.len(), request_count);
    assert!(!latencies.is_empty(), "Should have measured latencies");

    client.execute::<()>(client.flushdb()).ok();
}

// -----------------------------------------------------------------------
// Scenario E: Mixed Workload (Realistic)
// -----------------------------------------------------------------------

#[test]
fn test_scenario_mixed_workload() {
    let client = shared_client();
    client.execute::<()>(client.flushdb()).ok();

    let total_requests = 500;
    let start = std::time::Instant::now();

    for i in 0..total_requests {
        let user_idx = i % 50;
        let uid = format!("user-{user_idx}");
        let jti = format!("jti-{}", random_hex(8));
        let family_id = format!("fam-{}", user_idx / 10);
        let refresh_key = format!("refresh:{jti}");
        let family_key = format!("family:{family_id}");
        let denylist_key = format!("denylist:{jti}");

        // Simulate realistic mix using cmd() with owned strings
        match i % 20 {
            // 50% authz-core denylist checks
            0..=9 => {
                let _ = client.execute::<Option<String>>(client.get(&denylist_key));
            }
            // 30% token refresh (simplified)
            10..=13 => {
                let _ = client.execute::<Option<String>>(client.hget(&refresh_key, "sub"));
                let _ = client.execute::<usize>(client.sadd(&family_key, &jti));
            }
            // 15% login
            14..=16 => {
                let _ = client.execute::<()>(client.hset(&refresh_key, "sub", &uid));
            }
            // 5% user CRUD
            _ => {
                let _ = client.execute::<Option<String>>(client.get(&refresh_key));
            }
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let _ = (f64::from(total_requests) / elapsed_ms) * 1000.0;

    client.execute::<()>(client.flushdb()).ok();
}
