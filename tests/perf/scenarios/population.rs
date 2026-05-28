// Scenario A: User Population (2000 users)
//
// Populate Redis with realistic data for 2000 users.
//
// Operations per user:
// - 1 refresh:{jti} hash (8 fields)
// - 1 family:{family_id} set entry
// - 1 session:{sid} hash

use may::go;
use may_redis::RedisClient;
use may_redis::perf::{RefreshTokenData, generate_user_batch};

/// Populate N users into Redis.
///
/// # Arguments
/// * `count` — Number of users to populate (default 2000)
///
/// # Returns
/// `(total_keys_written, elapsed_ms)`
pub fn populate_users_scenario(
    count: usize,
    client: &RedisClient,
) -> Result<(usize, f64), String> {
    let users = generate_user_batch(count as u32, None);
    let start = std::time::Instant::now();
    let mut keys_written = 0usize;

    for user in &users {
        // HSET refresh:{jti} — 8 fields
        let fields = user.to_redis_fields();
        for (field, value) in &fields {
            client.hset(&format!("refresh:{}", user.jti), field, value)?;
            keys_written += 1;
        }

        // SADD family:{family_id} {jti}
        client.sadd(&format!("family:{}", user.family_id), user.jti.as_str())?;
        keys_written += 1;

        // SET session:{sid} with 30-day TTL
        let json = user.to_json();
        client.set_ex(&format!("session:{}", user.sid), &json, 2592000)?;
        keys_written += 1;
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok((keys_written, elapsed_ms))
}

/// Populate N users with concurrent coroutines.
///
/// Splits users across N workers, each handling a chunk.
///
/// # Arguments
/// * `count` — Total users to populate
/// * `workers` — Number of concurrent coroutines
/// * `client` — Redis client to write to
///
/// # Returns
/// `(total_keys, elapsed_ms)`
pub fn populate_concurrent_scenario(
    count: usize,
    workers: usize,
    client: &RedisClient,
) -> Result<(usize, f64), String> {
    let users = generate_user_batch(count as u32, None);
    let chunk_size = (count + workers - 1) / workers;
    let start = std::time::Instant::now();

    let mut handles = Vec::new();

    for chunk in users.chunks(chunk_size) {
        let client = client.clone();
        let handle = go!(move || {
            let mut keys = 0usize;
            for user in chunk {
                // HSET refresh:{jti}
                let fields = user.to_redis_fields();
                for (field, value) in &fields {
                    let _ = client.hset(&format!("refresh:{}", user.jti), field, value);
                    keys += 1;
                }

                // SADD family:{family_id} {jti}
                let _ = client.sadd(&format!("family:{}", user.family_id), user.jti.as_str());
                keys += 1;

                // SET session:{sid} with TTL
                let json = user.to_json();
                let _ = client.set_ex(&format!("session:{}", user.sid), &json, 2592000);
                keys += 1;
            }
            keys
        });
        handles.push(handle);
    }

    let total_keys: usize = handles.iter().map(|h| h.join()).sum();
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok((total_keys, elapsed_ms))
}
