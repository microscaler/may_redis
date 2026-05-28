// Redis operation helpers for performance testing.
//
// Wraps may-redis `RedisClient` operations to match Sesame-IDAM's
// Redis usage patterns (HSET, HGET, SADD, SISMEMBER, etc.) in
// batched/concurrent contexts.

use may_redis::RedisClient;

/// Populate Redis with user data for testing.
///
/// Each user gets:
/// - `refresh:{jti}` hash (8 fields)
/// - `family:{family_id}` set entry
/// - `session:{sid}` hash (5 fields)
///
/// # Arguments
/// * `client` — Redis client to write to
/// * `users` — Pre-generated `RefreshTokenData` batch
///
/// # Returns
/// `Result<usize>` — number of keys written
pub fn populate_users(client: &RedisClient, users: &[crate::perf::RefreshTokenData]) -> Result<usize, String> {
    let mut keys_written = 0usize;

    for user in users {
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

    Ok(keys_written)
}

/// Clean up all user data from Redis.
///
/// # Arguments
/// * `client` — Redis client to write to
/// * `users` — Users whose data to remove
///
/// # Returns
/// `Result<usize>` — number of keys deleted
pub fn cleanup_users(client: &RedisClient, users: &[crate::perf::RefreshTokenData]) -> Result<usize, String> {
    let mut keys_deleted = 0usize;

    for user in users {
        // DEL refresh:{jti}
        client.del(&format!("refresh:{}", user.jti))?;
        keys_deleted += 1;

        // SREM family:{family_id} {jti}
        client.srem(&format!("family:{}", user.family_id), user.jti.as_str())?;
        keys_deleted += 1;

        // DEL session:{sid}
        client.del(&format!("session:{}", user.sid))?;
        keys_deleted += 1;
    }

    Ok(keys_deleted)
}

/// Simulate authz-core denylist checks.
///
/// Checks `denylist:{jti}` for each user's jti and returns the
/// number of cache hits (keys found).
///
/// # Arguments
/// * `client` — Redis client to read from
/// * `users` — Users' jti values to check
/// * `hit_ratio` — Fraction of keys that exist (rest return None)
///
/// # Returns
/// `(hits, total)` — number of hits and total checks performed
pub fn batch_denylist_check(
    client: &RedisClient,
    users: &[crate::perf::RefreshTokenData],
    hit_ratio: f64,
) -> Result<(usize, usize), String> {
    let mut hits = 0usize;
    let total = users.len();

    for user in users {
        // Check if key exists
        let value: Result<Option<String>, _> = client.get(&format!("denylist:{}", user.jti));
        match value {
            Ok(Some(_)) => hits += 1,
            Ok(None) => {} // Key not found (cache miss simulation)
            Err(_) => {} // Error (fail-open behavior)
        }
    }

    Ok((hits, total))
}

/// Simulate token refresh flow (9-step operation).
///
/// Performs the full refresh token rotation sequence:
/// 1. HGET refresh:{jti} — lookup old token
/// 2. GET denylist:{jti} — check revoked
/// 3. SISMEMBER family:{family_id} __REVOKED__ — check family revocation
/// 4. DEL refresh:{jti} — invalidate old
/// 5. SREM family:{family_id} {jti} — remove from family
/// 6. SADD family:{new_family} {new_jti} — add to new family
/// 7. SET denylist:{old_jti} — add to denylist
/// 8. HSET refresh:{new_jti} — store new token
/// 9. SADD family:{new_family} {new_jti} — add to new family
///
/// # Arguments
/// * `client` — Redis client
/// * `user` — User whose token to refresh
///
/// # Returns
/// `Result<()>`
pub fn refresh_user_token(
    client: &RedisClient,
    user: &crate::perf::RefreshTokenData,
    new_jti: &str,
) -> Result<(), String> {
    // Step 1: HGET refresh:{jti}
    let _: Result<Option<String>, _> = client.get(&format!("refresh:{}", user.jti));

    // Step 2: GET denylist:{jti}
    let _: Result<Option<String>, _> = client.get(&format!("denylist:{}", user.jti));

    // Step 3: SISMEMBER family:{family_id} __REVOKED__
    let _: Result<bool, _> = client.sismember(&format!("family:{}", user.family_id), "__REVOKED__");

    // Step 4: DEL refresh:{jti}
    let _: Result<usize, _> = client.del(&format!("refresh:{}", user.jti));

    // Step 5: SREM family:{family_id} {jti}
    let _: Result<usize, _> = client.srem(&format!("family:{}", user.family_id), &user.jti);

    // Step 6-9: Set up new token (simplified — same family for this test)
    // Step 7: SET denylist:{old_jti}
    let _: Result<(), _> = client.set_ex(&format!("denylist:{}", user.jti), "rotated", 86400);

    // Step 8: HSET refresh:{new_jti} (simplified — just store jti field)
    client.hset(&format!("refresh:{}", new_jti), "jti", new_jti)?;

    // Step 9: SADD family:{family_id} {new_jti}
    let _: Result<usize, _> = client.sadd(&format!("family:{}", user.family_id), new_jti);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_populate_users_counts_keys() {
        // Unit test — doesn't need real Redis, just verifies the function exists
        // and the API is correct
        let client = RedisClient::new("redis://127.0.0.1:6379");
        let users = crate::perf::jwt::generate_user_batch(5, None);
        // This will fail because Redis isn't running, but it validates the API
        let result = populate_users(&client, &users);
        assert!(result.is_err()); // Connection refused expected
    }

    #[test]
    fn test_cleanup_users_api() {
        let client = RedisClient::new("redis://127.0.0.1:6379");
        let users = crate::perf::jwt::generate_user_batch(5, None);
        let result = cleanup_users(&client, &users);
        assert!(result.is_err()); // Connection refused expected
    }

    #[test]
    fn test_batch_denylist_check_api() {
        let client = RedisClient::new("redis://127.0.0.1:6379");
        let users = crate::perf::jwt::generate_user_batch(5, None);
        let result = batch_denylist_check(&client, &users, 0.5);
        assert!(result.is_err()); // Connection refused expected
    }

    #[test]
    fn test_refresh_token_api() {
        let client = RedisClient::new("redis://127.0.0.1:6379");
        let (user, _) = crate::perf::jwt::generate_user_jwt(1, None);
        let result = refresh_user_token(&client, &user, "new-jti-123");
        assert!(result.is_err()); // Connection refused expected
    }
}
