// JWT claim generation for performance testing.
//
// Generates realistic JWT-like payloads matching Sesame-IDAM's
// refresh token structure for benchmarking Redis operations.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Refresh token payload stored in Redis `refresh:{jti}` hash.
///
/// Mirrors sesame-idam's `RefreshToken` model from
/// `microservices/idam/identity-session-service/impl/src/models/refresh_token.rs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshTokenData {
    /// Unique token ID (also in access token denylist)
    pub jti: String,
    /// User ID (subject)
    pub sub: String,
    /// Session ID
    pub sid: String,
    /// Token family identifier (for reuse detection)
    pub family_id: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Client application identifier
    pub client_id: String,
    /// Space-delimited scopes
    pub scopes: String,
}

impl RefreshTokenData {
    /// Create a new `RefreshTokenData` instance.
    #[must_use]
    pub fn new(
        jti: String,
        sub: String,
        sid: String,
        family_id: String,
        iat: i64,
        exp: i64,
        client_id: String,
        scopes: String,
    ) -> Self {
        Self {
            jti,
            sub,
            sid,
            family_id,
            iat,
            exp,
            client_id,
            scopes,
        }
    }

    /// Convert to a map of field name -> value for Redis HSET.
    #[must_use]
    pub fn to_redis_fields(&self) -> Vec<(&str, &str)> {
        vec![
            ("jti", &self.jti),
            ("sub", &self.sub),
            ("sid", &self.sid),
            ("family_id", &self.family_id),
            ("iat", &self.iat.to_string()),
            ("exp", &self.exp.to_string()),
            ("client_id", &self.client_id),
            ("scopes", &self.scopes),
        ]
    }

    /// Serialize to JSON for session storage.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Generate a JWT-like payload for a single user.
///
/// # Arguments
/// * `user_id` — Unique user identifier
/// * `access_ttl_secs` — Access token TTL in seconds (default 3600)
///
/// # Returns
/// A tuple of `(RefreshTokenData, String)` where the string is the
/// base64-encoded payload for reference.
#[must_use]
pub fn generate_user_jwt(
    user_id: u32,
    access_ttl_secs: Option<u64>,
) -> (RefreshTokenData, String) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let exp = now + (access_ttl_secs.unwrap_or(3600)) as i64;

    let jti = format!("jti-{}-{}", user_id, generate_random_hex(16));
    let sub = format!("user-{}", user_id);
    let sid = format!("sid-{}-{}", user_id, generate_random_hex(8));
    let family_id = format!("fam-{}", user_id / 10); // 10 users per family

    let data = RefreshTokenData::new(
        jti,
        sub.clone(),
        sid.clone(),
        family_id,
        now,
        exp,
        "web-app".to_string(),
        "read write".to_string(),
    );

    let payload = serde_json::json!({
        "sub": sub,
        "iss": "https://idam.example.com",
        "exp": exp,
        "iat": now,
        "nbf": now,
        "jti": data.jti,
        "scope": "read write",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "user_id": user_id.to_string(),
    });

    (data, payload.to_string())
}

/// Generate multiple user JWT payloads for batch population.
///
/// # Arguments
/// * `count` — Number of users to generate
/// * `access_ttl_secs` — Access token TTL in seconds
///
/// # Returns
/// A vector of `RefreshTokenData` for all generated users.
#[must_use]
pub fn generate_user_batch(count: u32, access_ttl_secs: Option<u64>) -> Vec<RefreshTokenData> {
    (0..count)
        .map(|i| generate_user_jwt(i, access_ttl_secs).0)
        .collect()
}

/// Generate a random hex string of the specified length.
fn generate_random_hex(len: usize) -> String {
    (0..len)
        .map(|_| format!("{:02x}", fastrand::u8(0..=255)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_single_user_jwt() {
        let (data, payload) = generate_user_jwt(1, Some(3600));
        assert_eq!(data.sub, "user-1");
        assert_eq!(data.jti, "jti-1-");
        assert!(data.jti.len() > 10);
        assert!(!payload.is_empty());
        assert_eq!(data.scopes, "read write");
        assert_eq!(data.client_id, "web-app");
    }

    #[test]
    fn test_generate_unique_users() {
        let user1 = generate_user_jwt(1, None).0;
        let user2 = generate_user_jwt(2, None).0;
        assert_ne!(user1.jti, user2.jti);
        assert_eq!(user1.sub, "user-1");
        assert_eq!(user2.sub, "user-2");
        // Same family for users 1-10
        assert_eq!(user1.family_id, user2.family_id);
    }

    #[test]
    fn test_generate_batch() {
        let batch = generate_user_batch(10, None);
        assert_eq!(batch.len(), 10);
        for (i, data) in batch.iter().enumerate() {
            assert_eq!(data.sub, format!("user-{}", i));
        }
    }

    #[test]
    fn test_to_redis_fields() {
        let (data, _) = generate_user_jwt(42, None);
        let fields = data.to_redis_fields();
        assert_eq!(fields.len(), 8);
        assert_eq!(fields[0].0, "jti");
        assert_eq!(fields[1].0, "sub");
        assert!(fields.iter().any(|(k, _)| *k == "exp"));
    }

    #[test]
    fn test_to_json_serialization() {
        let (data, _) = generate_user_jwt(1, None);
        let json = data.to_json();
        let parsed: RefreshTokenData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.jti, data.jti);
        assert_eq!(parsed.sub, data.sub);
    }

    #[test]
    fn test_tenant_id_from_design_doc() {
        // Validate we use the same tenant_id as sesame-idam's design doc
        let payload = generate_user_jwt(1, None).1;
        assert!(payload.contains("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
    }
}
