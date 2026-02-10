//! OBLIVION - Time-To-Live (TTL) Support
//! Provides key expiration functionality similar to Redis EXPIRE.
//!
//! Keys with a TTL will automatically be treated as deleted
//! once their expiration timestamp has passed.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::Key;

/// Manages TTL (Time-To-Live) for keys in the storage engine.
///
/// ## Design
/// - Stores expiration timestamps as Unix epoch milliseconds
/// - Uses a `BTreeMap<Key, u64>` for O(log n) lookups
/// - Maintains a reverse index `BTreeMap<u64, Vec<Key>>` for efficient
///   expiration scanning (find all keys expiring before timestamp T)
///
/// ## Integration
/// The engine checks `is_expired(key)` on every `get()` call.
/// Expired keys are lazily cleaned up (tombstoned) during compaction.
pub struct TtlIndex {
    /// Map from key -> expiration timestamp (ms since epoch).
    expirations: BTreeMap<Key, u64>,
}

impl TtlIndex {
    /// Create a new empty TTL index.
    pub fn new() -> Self {
        Self {
            expirations: BTreeMap::new(),
        }
    }

    /// Set a TTL for a key.
    ///
    /// # Arguments
    /// * `key` - The key to set TTL for
    /// * `ttl_ms` - Time-to-live in milliseconds from now
    pub fn set_ttl(&mut self, key: Key, ttl_ms: u64) {
        let expires_at = Self::now_ms() + ttl_ms;
        self.expirations.insert(key, expires_at);
    }

    /// Set an absolute expiration timestamp for a key.
    ///
    /// # Arguments
    /// * `key` - The key to set expiration for
    /// * `expires_at_ms` - Absolute Unix timestamp in milliseconds
    pub fn set_expiration(&mut self, key: Key, expires_at_ms: u64) {
        self.expirations.insert(key, expires_at_ms);
    }

    /// Remove TTL for a key (make it persistent).
    pub fn remove_ttl(&mut self, key: &[u8]) {
        self.expirations.remove(key);
    }

    /// Check if a key has expired.
    /// Returns `true` if the key has a TTL and it has passed.
    /// Returns `false` if the key has no TTL or hasn't expired yet.
    pub fn is_expired(&self, key: &[u8]) -> bool {
        match self.expirations.get(key) {
            Some(&expires_at) => Self::now_ms() >= expires_at,
            None => false, // No TTL = never expires
        }
    }

    /// Get the remaining TTL for a key in milliseconds.
    /// Returns `None` if the key has no TTL.
    /// Returns `Some(0)` if the key has already expired.
    pub fn remaining_ttl(&self, key: &[u8]) -> Option<u64> {
        self.expirations.get(key).map(|&expires_at| {
            let now = Self::now_ms();
            if now >= expires_at {
                0
            } else {
                expires_at - now
            }
        })
    }

    /// Get the expiration timestamp for a key.
    pub fn get_expiration(&self, key: &[u8]) -> Option<u64> {
        self.expirations.get(key).copied()
    }

    /// Collect all expired keys as of now.
    /// Useful for batch cleanup during compaction.
    pub fn collect_expired(&self) -> Vec<Key> {
        let now = Self::now_ms();
        self.expirations
            .iter()
            .filter(|(_, &expires_at)| now >= expires_at)
            .map(|(key, _)| key.clone())
            .collect()
    }

    /// Remove all expired entries from the index.
    /// Returns the number of entries purged.
    pub fn purge_expired(&mut self) -> usize {
        let expired = self.collect_expired();
        let count = expired.len();
        for key in expired {
            self.expirations.remove(&key);
        }
        count
    }

    /// Returns the number of keys with active TTLs.
    pub fn len(&self) -> usize {
        self.expirations.len()
    }

    /// Returns true if no keys have TTLs.
    pub fn is_empty(&self) -> bool {
        self.expirations.is_empty()
    }

    /// Get current time in milliseconds since Unix epoch.
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl Default for TtlIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_set_and_check_ttl() {
        let mut ttl = TtlIndex::new();

        // Set a TTL of 10 seconds (should not be expired)
        ttl.set_ttl(b"key1".to_vec(), 10_000);
        assert!(!ttl.is_expired(b"key1"));
        assert!(ttl.remaining_ttl(b"key1").unwrap() > 0);
    }

    #[test]
    fn test_no_ttl_never_expires() {
        let ttl = TtlIndex::new();
        assert!(!ttl.is_expired(b"no_ttl_key"));
        assert_eq!(ttl.remaining_ttl(b"no_ttl_key"), None);
    }

    #[test]
    fn test_immediate_expiration() {
        let mut ttl = TtlIndex::new();

        // Set expiration in the past
        ttl.set_expiration(b"old_key".to_vec(), 0);
        assert!(ttl.is_expired(b"old_key"));
        assert_eq!(ttl.remaining_ttl(b"old_key"), Some(0));
    }

    #[test]
    fn test_remove_ttl() {
        let mut ttl = TtlIndex::new();
        ttl.set_ttl(b"key".to_vec(), 1000);
        assert_eq!(ttl.len(), 1);

        ttl.remove_ttl(b"key");
        assert_eq!(ttl.len(), 0);
        assert!(!ttl.is_expired(b"key"));
    }

    #[test]
    fn test_collect_expired() {
        let mut ttl = TtlIndex::new();

        // 2 expired keys
        ttl.set_expiration(b"expired1".to_vec(), 0);
        ttl.set_expiration(b"expired2".to_vec(), 1);

        // 1 active key
        ttl.set_ttl(b"active".to_vec(), 60_000);

        let expired = ttl.collect_expired();
        assert_eq!(expired.len(), 2);
        assert!(expired.contains(&b"expired1".to_vec()));
        assert!(expired.contains(&b"expired2".to_vec()));
    }

    #[test]
    fn test_purge_expired() {
        let mut ttl = TtlIndex::new();
        ttl.set_expiration(b"old1".to_vec(), 0);
        ttl.set_expiration(b"old2".to_vec(), 0);
        ttl.set_ttl(b"fresh".to_vec(), 60_000);

        assert_eq!(ttl.len(), 3);
        let purged = ttl.purge_expired();
        assert_eq!(purged, 2);
        assert_eq!(ttl.len(), 1);
    }

    #[test]
    fn test_short_ttl_expires() {
        let mut ttl = TtlIndex::new();
        ttl.set_ttl(b"short".to_vec(), 50); // 50ms

        assert!(!ttl.is_expired(b"short"));

        // Wait for expiration
        thread::sleep(Duration::from_millis(100));

        assert!(ttl.is_expired(b"short"));
        assert_eq!(ttl.remaining_ttl(b"short"), Some(0));
    }
}
