// InMemoryClient test suite — all tests extracted from in_memory.rs
//
// Covers: basic CRUD, TTL expiry/edge cases, INCR edge cases,
// KEYS glob patterns, JSON string values, large values,
// binary/non-UTF-8, clone sharing.

use crate::client::in_memory::InMemoryClient;
use crate::core::RedisError;

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inmemory_set_get() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        assert_eq!(client.get("key").unwrap(), "value");
    }

    #[test]
    fn test_inmemory_set_ex_get() {
        let client = InMemoryClient::new();
        client.set_ex("key", "value", 60);
        assert_eq!(client.get("key").unwrap(), "value");
    }

    #[test]
    fn test_inmemory_del() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        assert_eq!(client.del("key").unwrap(), 1);
        assert_eq!(client.del("missing").unwrap(), 0);
    }

    #[test]
    fn test_inmemory_exists() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        assert!(client.exists("key").unwrap());
        assert!(!client.exists("missing").unwrap());
    }

    #[test]
    fn test_inmemory_incr() {
        let client = InMemoryClient::new();
        assert_eq!(client.incr("counter").unwrap(), 1);
        assert_eq!(client.incr("counter").unwrap(), 2);
        assert_eq!(client.incr("counter").unwrap(), 3);
    }

    #[test]
    fn test_inmemory_incr_on_string_error() {
        let client = InMemoryClient::new();
        client.set("key", "not_a_number");
        assert!(client.incr("key").is_err());
    }

    #[test]
    fn test_inmemory_ttl() {
        let client = InMemoryClient::new();
        client.set_ex("key", "value", 60);
        let ttl = client.ttl("key").unwrap();
        assert!(ttl > 0 && ttl <= 60);
    }

    #[test]
    fn test_inmemory_expire() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        // No TTL initially
        assert_eq!(client.ttl("key").unwrap(), -1);
        // Set TTL
        assert!(client.expire("key", 30).unwrap());
        let ttl = client.ttl("key").unwrap();
        assert!(ttl > 0 && ttl <= 30);
    }

    #[test]
    fn test_inmemory_keys_pattern() {
        let client = InMemoryClient::new();
        client.set("user:1", "alice");
        client.set("user:2", "bob");
        client.set("other:1", "x");
        let keys = client.keys("user:*").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"user:2".to_string()));
    }

    #[test]
    fn test_inmemory_dbsize() {
        let client = InMemoryClient::new();
        assert_eq!(client.dbsize().unwrap(), 0);
        client.set("a", "1");
        client.set("b", "2");
        assert_eq!(client.dbsize().unwrap(), 2);
    }

    #[test]
    fn test_inmemory_flushdb() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        client.flushdb();
        assert_eq!(client.dbsize().unwrap(), 0);
        // Missing key returns Ok("") (Null in RESP), not error
        assert!(client.get("key").is_ok());
        assert_eq!(client.get("key").unwrap(), "");
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("user:*", "user:1"));
        assert!(glob_match("user:*", "user:abc"));
        assert!(!glob_match("user:*", "other:1"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("?", "a"));
        assert!(!glob_match("?", "ab"));
        assert!(glob_match("user:?", "user:1"));
        assert!(!glob_match("user:?", "user:12"));
    }

    #[test]
    fn test_glob_match_literal() {
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exact2"));
    }

    #[test]
    fn test_clone_shares_store() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        let cloned = client.clone();
        assert_eq!(cloned.get("key").unwrap(), "value");
        client.set("key", "new_value");
        assert_eq!(cloned.get("key").unwrap(), "new_value");
    }

    // -----------------------------------------------------------------------
    // TTL edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_expired_key_returns_null() {
        let client = InMemoryClient::new();
        client.set_ex("key", "value", 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        // Expired keys return Ok("") (Null in RESP), matching real Redis
        assert!(client.get("key").is_ok());
        assert_eq!(client.get("key").unwrap(), "");
    }

    #[test]
    fn test_exists_expired_key_returns_false() {
        let client = InMemoryClient::new();
        client.set_ex("key", "value", 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!client.exists("key").unwrap());
    }

    #[test]
    fn test_ttl_no_ttl_returns_negative_one() {
        let client = InMemoryClient::new();
        client.set("key", "value");
        assert_eq!(client.ttl("key").unwrap(), -1);
    }

    #[test]
    fn test_ttl_missing_key_returns_error() {
        let client = InMemoryClient::new();
        assert!(client.ttl("missing").is_err());
    }

    #[test]
    fn test_expire_on_missing_key_returns_false() {
        let client = InMemoryClient::new();
        assert!(!client.expire("missing", 60).unwrap());
    }

    #[test]
    fn test_del_missing_key_returns_zero() {
        let client = InMemoryClient::new();
        assert_eq!(client.del("missing").unwrap(), 0);
    }

    #[test]
    fn test_flushdb_clears_everything() {
        let client = InMemoryClient::new();
        client.set("a", "1");
        client.set_ex("b", "2", 60);
        client.flushdb();
        assert_eq!(client.dbsize().unwrap(), 0);
        assert!(client.get("a").is_ok());
        assert!(client.get("b").is_ok());
    }

    // -----------------------------------------------------------------------
    // INCR edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_incr_missing_key_creates_one() {
        let client = InMemoryClient::new();
        assert_eq!(client.incr("missing").unwrap(), 1);
        assert_eq!(client.get("missing").unwrap(), "1");
    }

    #[test]
    fn test_incr_preserves_ttl() {
        let client = InMemoryClient::new();
        client.set_ex("counter", "0", 60);
        let result = client.incr("counter").unwrap();
        assert_eq!(result, 1);
        // TTL should still be set after INCR
        let ttl = client.ttl("counter").unwrap();
        assert!(ttl > 0 && ttl <= 60);
    }

    #[test]
    fn test_incr_on_negative_value() {
        let client = InMemoryClient::new();
        client.set("neg", "-10");
        assert_eq!(client.incr("neg").unwrap(), -9);
        assert_eq!(client.incr("neg").unwrap(), -8);
    }

    #[test]
    fn test_incr_on_empty_string() {
        let client = InMemoryClient::new();
        client.set("empty", "");
        assert!(client.incr("empty").is_err());
    }

    // -----------------------------------------------------------------------
    // KEYS glob patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_keys_match_all() {
        let client = InMemoryClient::new();
        client.set("a", "1");
        client.set("ab", "2");
        client.set("abc", "3");
        let keys = client.keys("*").unwrap();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_keys_pattern_prefix() {
        let client = InMemoryClient::new();
        client.set("user:1", "alice");
        client.set("user:2", "bob");
        client.set("admin:1", "root");
        let keys = client.keys("user:*").unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_keys_pattern_single_char() {
        let client = InMemoryClient::new();
        client.set("a1", "1");
        client.set("a2", "2");
        client.set("b1", "3");
        let keys = client.keys("a?").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"a1".to_string()));
        assert!(keys.contains(&"a2".to_string()));
    }

    #[test]
    fn test_keys_no_match_returns_empty() {
        let client = InMemoryClient::new();
        client.set("a", "1");
        client.set("b", "2");
        let keys = client.keys("x:*").unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_keys_empty_store() {
        let client = InMemoryClient::new();
        let keys = client.keys("*").unwrap();
        assert!(keys.is_empty());
    }

    // -----------------------------------------------------------------------
    // JSON string values (stored as plain strings)
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_json_object_string() {
        let client = InMemoryClient::new();
        let json = r#"{"name":"alice","age":30,"active":true}"#;
        client.set("user:1", json);
        assert_eq!(client.get("user:1").unwrap(), json);
    }

    #[test]
    fn test_set_json_array_string() {
        let client = InMemoryClient::new();
        let json = r#"["alpha","beta","gamma"]"#;
        client.set("arr", json);
        assert_eq!(client.get("arr").unwrap(), json);
    }

    #[test]
    fn test_set_nested_json() {
        let client = InMemoryClient::new();
        let json = r#"{"user":{"name":"bob","prefs":{"theme":"dark"}}}"#;
        client.set("user:2", json);
        assert_eq!(client.get("user:2").unwrap(), json);
    }

    #[test]
    fn test_set_json_null_value() {
        let client = InMemoryClient::new();
        client.set("null_val", "null");
        assert_eq!(client.get("null_val").unwrap(), "null");
    }

    #[test]
    fn test_set_json_with_special_chars() {
        let client = InMemoryClient::new();
        let json = r#"{"msg":"hello \"world\" \n\t tab"}"#;
        client.set("special", json);
        assert_eq!(client.get("special").unwrap(), json);
    }

    #[test]
    fn test_set_json_with_unicode() {
        let client = InMemoryClient::new();
        let json = r#"{"emoji":"😀","chinese":"你好","japanese":"こんにちは"}"#;
        client.set("unicode", json);
        assert_eq!(client.get("unicode").unwrap(), json);
    }

    #[test]
    fn test_json_with_ttl() {
        let client = InMemoryClient::new();
        let json = r#"{"key":"value"}"#;
        client.set_ex("json:ttl", json, 60);
        assert_eq!(client.get("json:ttl").unwrap(), json);
        let ttl = client.ttl("json:ttl").unwrap();
        assert!(ttl > 0 && ttl <= 60);
    }

    // -----------------------------------------------------------------------
    // Large values
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_large_value() {
        let client = InMemoryClient::new();
        let large = "x".repeat(100_000);
        client.set("large", &large);
        assert_eq!(client.get("large").unwrap(), large);
    }

    #[test]
    fn test_set_many_keys() {
        let client = InMemoryClient::new();
        for i in 0..1000 {
            client.set(&format!("key:{i}"), &i.to_string());
        }
        assert_eq!(client.dbsize().unwrap(), 1000);
    }

    // -----------------------------------------------------------------------
    // Binary / non-UTF-8 values
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_empty_string() {
        let client = InMemoryClient::new();
        client.set("empty", "");
        assert_eq!(client.get("empty").unwrap(), "");
    }

    #[test]
    fn test_get_nonexistent_key() {
        let client = InMemoryClient::new();
        assert!(client.get("nope").is_ok());
    }

    // -----------------------------------------------------------------------
    // Clone sharing
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_multiple_writers() {
        let client = InMemoryClient::new();
        let c1 = client.clone();
        let c2 = client.clone();
        let c3 = client.clone();
        c1.set("shared", "from-c1");
        c2.set("shared", "from-c2");
        c3.set("shared", "from-c3");
        assert_eq!(client.get("shared").unwrap(), "from-c3");
    }

    // -----------------------------------------------------------------------
    // TTL expiry race with KEYS
    // -----------------------------------------------------------------------

    #[test]
    fn test_keys_excludes_expired() {
        let client = InMemoryClient::new();
        client.set("live", "1");
        client.set_ex("dead", "2", 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let keys = client.keys("*").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"live".to_string()));
    }
}
