// InMemoryClient — In-memory Redis backend for test isolation
//
// Provides `InMemoryClient` and `InMemoryStore` for testing without
// a running Redis server. Implements a subset of Redis commands with
// TTL support and glob-based KEYS matching.
//
// This module is only compiled when the `test` feature is enabled.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::core::RedisError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Internal store backing `InMemoryClient`.
///
/// Stores key-value pairs with optional TTL expiry. TTLs are checked
/// lazily on access (GET, EXISTS, TTL, INCR).
pub struct InMemoryStore {
    data: HashMap<String, (String, Option<Instant>)>,
}

impl InMemoryStore {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Remove expired entries. Called lazily before key access.
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.data
            .retain(|_, (_, expiry)| expiry.is_none_or(|e| e > now));
    }

    /// Get a value, returning Ok("") if missing or expired.
    ///
    /// # Errors
    /// This is an infallible variant — returns an empty `String` for missing
    /// keys rather than an error.
    pub fn get(&mut self, key: &str) -> Result<String, RedisError> {
        self.cleanup();
        match self.data.get(key) {
            Some((value, _)) => Ok(value.clone()),
            None => Ok(String::new()),
        }
    }

    /// Set a value without TTL.
    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), (value.to_string(), None));
    }

    /// Set a value with TTL in seconds.
    pub fn set_ex(&mut self, key: &str, value: &str, seconds: u64) {
        let expiry = Instant::now() + std::time::Duration::from_secs(seconds);
        self.data
            .insert(key.to_string(), (value.to_string(), Some(expiry)));
    }

    /// Delete a key, returning the number of keys deleted (0 or 1).
    pub fn del(&mut self, key: &str) -> usize {
        self.data.remove(key).map_or(0, |_| 1)
    }

    /// Check if a key exists and is not expired.
    pub fn exists(&mut self, key: &str) -> bool {
        self.cleanup();
        self.data.contains_key(key)
    }

    /// Atomically increment an integer value.
    ///
    /// - On a missing key: auto-create with value 1.
    /// - On a non-integer value: return error.
    ///
    /// # Errors
    /// Returns [`RedisError::Other`] if the existing value is not a valid integer.
    pub fn incr(&mut self, key: &str) -> Result<i64, RedisError> {
        self.cleanup();
        let current = self.data.get(key).map(|(v, _)| v.clone());
        let new_val = match current {
            Some(s) => {
                let n: i64 = s
                    .parse()
                    .map_err(|_| RedisError::Other("ERR value is not an integer".to_string()))?;
                n + 1
            }
            None => 1,
        };
        // Preserve any existing TTL
        let expiry = self.data.get(key).and_then(|(_, e)| *e);
        self.data
            .insert(key.to_string(), (new_val.to_string(), expiry));
        Ok(new_val)
    }

    /// Get remaining TTL in seconds.
    ///
    /// # Errors
    /// Returns [`RedisError::Other`] if the key does not exist.
    pub fn ttl(&mut self, key: &str) -> Result<i64, RedisError> {
        self.cleanup();
        match self.data.get(key) {
            Some((_, Some(expiry))) => {
                let remaining = expiry.saturating_duration_since(Instant::now());
                Ok(remaining.as_secs().cast_signed())
            }
            Some((_, None)) => Ok(-1),
            None => Err(RedisError::Other(format!("no such key: {key}"))),
        }
    }

    /// Set TTL on an existing key.
    pub fn expire(&mut self, key: &str, seconds: u64) -> bool {
        self.cleanup();
        if self.data.contains_key(key) {
            let expiry = Instant::now() + std::time::Duration::from_secs(seconds);
            if let Some(entry) = self.data.get_mut(key) {
                entry.1 = Some(expiry);
            }
            true
        } else {
            false
        }
    }

    /// Get matching keys with glob support (`*` and `?*`).
    ///
    /// Expired keys are lazily removed before enumeration so they don't
    /// appear in results — matching real Redis behaviour.
    #[must_use]
    pub fn keys(&mut self, pattern: &str) -> Vec<String> {
        self.cleanup();
        let mut result = Vec::new();
        for key in self.data.keys() {
            if glob_match(pattern, key) {
                result.push(key.clone());
            }
        }
        result
    }

    /// Get the number of keys in the store.
    #[must_use]
    pub fn dbsize(&self) -> usize {
        self.data.len()
    }

    /// Clear all data.
    pub fn flushdb(&mut self) {
        self.data.clear();
    }
}

/// Simple glob matching supporting `*` (match anything) and `?*` (single char wildcard).
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let pat_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut star_pos: Option<usize> = None;
    let mut text_backtrack = 0usize;

    while ti < text_chars.len() {
        if pi < pat_chars.len() && (pat_chars[pi] == '?' || pat_chars[pi] == text_chars[ti]) {
            // Match single char
            pi += 1;
            ti += 1;
        } else if pi < pat_chars.len() && pat_chars[pi] == '*' {
            // Star match: record position, try matching zero chars next
            star_pos = Some(pi);
            text_backtrack = ti;
            pi += 1;
        } else if let Some(star) = star_pos {
            // Backtrack: star matches one more char
            pi = star + 1;
            text_backtrack += 1;
            ti = text_backtrack;
        } else {
            return false;
        }
    }

    // Remaining pattern chars must all be '*'
    while pi < pat_chars.len() && pat_chars[pi] == '*' {
        pi += 1;
    }

    pi == pat_chars.len()
}

/// In-memory Redis client for test isolation.
///
/// Thread-safe: protected by `Arc<Mutex<InMemoryStore>>`.
/// No dependency on `may` runtime or a running Redis server.
///
/// # Panics
/// All methods panic if the `Arc<Mutex<InMemoryStore>>` is poisoned by a
/// previous panic in another caller holding the lock.
pub struct InMemoryClient {
    store: Arc<Mutex<InMemoryStore>>,
}

impl Default for InMemoryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryClient {
    /// Create a new empty in-memory client.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(InMemoryStore::new())),
        }
    }

    /// GET key.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned by a previous panic.
    pub fn get(&self, key: &str) -> Result<String, RedisError> {
        self.store.lock().unwrap().get(key)
    }

    /// SET key value
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn set(&self, key: &str, value: &str) {
        self.store.lock().unwrap().set(key, value);
    }

    /// SET key value EX seconds
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn set_ex(&self, key: &str, value: &str, seconds: u64) {
        self.store.lock().unwrap().set_ex(key, value, seconds);
    }

    /// DEL key — returns 0 or 1.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn del(&self, key: &str) -> Result<usize, RedisError> {
        Ok(self.store.lock().unwrap().del(key))
    }

    /// EXISTS key.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn exists(&self, key: &str) -> Result<bool, RedisError> {
        Ok(self.store.lock().unwrap().exists(key))
    }

    /// INCR key.
    ///
    /// # Errors
    /// Returns [`RedisError::Other`] if the value is not a valid integer, or
    /// [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn incr(&self, key: &str) -> Result<i64, RedisError> {
        self.store.lock().unwrap().incr(key)
    }

    /// TTL key.
    ///
    /// # Errors
    /// Returns [`RedisError::Other`] if the key does not exist, or
    /// [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn ttl(&self, key: &str) -> Result<i64, RedisError> {
        self.store.lock().unwrap().ttl(key)
    }

    /// EXPIRE key seconds.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn expire(&self, key: &str, seconds: u64) -> Result<bool, RedisError> {
        Ok(self.store.lock().unwrap().expire(key, seconds))
    }

    /// KEYS pattern — glob matching with `*` and `?*`.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn keys(&self, pattern: &str) -> Result<Vec<String>, RedisError> {
        Ok(self.store.lock().unwrap().keys(pattern))
    }

    /// DBSIZE.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the mutex is poisoned.
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn dbsize(&self) -> Result<usize, RedisError> {
        Ok(self.store.lock().unwrap().dbsize())
    }

    /// FLUSHDB
    ///
    /// # Panics
    /// If the `Arc<Mutex<InMemoryStore>>` is poisoned.
    pub fn flushdb(&self) {
        self.store.lock().unwrap().flushdb();
    }
}

impl Clone for InMemoryClient {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod in_memory_tests;
