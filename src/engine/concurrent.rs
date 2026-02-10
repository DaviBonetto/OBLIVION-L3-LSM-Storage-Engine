//! OBLIVION - Concurrent Engine Wrapper
//! Thread-safe wrapper around the Oblivion engine using Arc + RwLock.
//!
//! ## Concurrency Model
//! - **Read operations** (`get`, `scan`, `len`, etc.) acquire a **read lock** (shared)
//! - **Write operations** (`put`, `delete`) acquire a **write lock** (exclusive)
//! - Multiple concurrent readers allowed, writers block all
//!
//! ## Use Case
//! This wrapper enables safe concurrent access to the engine from multiple threads,
//! making it suitable for server applications with concurrent client requests.

use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::error::Result;
use crate::types::{Key, Value};

use super::metrics::EngineMetrics;
use super::Oblivion;

/// Thread-safe wrapper around the Oblivion storage engine.
///
/// ## Example
/// ```no_run
/// use oblivion::engine::concurrent::ConcurrentOblivion;
/// use oblivion::config::Config;
/// use std::thread;
///
/// let config = Config::default();
/// let engine = ConcurrentOblivion::open(config).unwrap();
///
/// // Clone for multiple threads
/// let engine_clone = engine.clone();
///
/// // Thread 1: Write
/// thread::spawn(move || {
///     engine_clone.put(b"key".to_vec(), b"value".to_vec()).unwrap();
/// });
///
/// // Thread 2: Read
/// let result = engine.get(b"key");
/// ```
#[derive(Clone)]
pub struct ConcurrentOblivion {
    inner: Arc<RwLock<Oblivion>>,
}

impl ConcurrentOblivion {
    /// Open or create a concurrent Oblivion storage engine.
    pub fn open(config: Config) -> Result<Self> {
        let engine = Oblivion::open(config)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(engine)),
        })
    }

    /// Insert a key-value pair (write lock).
    pub fn put(&self, key: Key, value: Value) -> Result<()> {
        self.inner.write().unwrap().put(key, value)
    }

    /// Insert a key-value pair with TTL (write lock).
    pub fn put_with_ttl(&self, key: Key, value: Value, ttl_ms: u64) -> Result<()> {
        self.inner.write().unwrap().put_with_ttl(key, value, ttl_ms)
    }

    /// Get a value by key (read lock).
    pub fn get(&self, key: &[u8]) -> Option<Value> {
        self.inner.read().unwrap().get(key)
    }

    /// Delete a key (write lock).
    pub fn delete(&self, key: Key) -> Result<()> {
        self.inner.write().unwrap().delete(key)
    }

    /// Scan all key-value pairs (read lock).
    pub fn scan(&self) -> Vec<(Key, Value)> {
        self.inner.read().unwrap().scan()
    }

    /// Get remaining TTL for a key (read lock).
    pub fn ttl(&self, key: &[u8]) -> Option<u64> {
        self.inner.read().unwrap().ttl(key)
    }

    /// Get number of entries (read lock).
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// Check if engine is empty (read lock).
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    /// Get MemTable size in bytes (read lock).
    pub fn memtable_size(&self) -> usize {
        self.inner.read().unwrap().memtable_size()
    }

    /// Get a snapshot of the engine metrics (read lock).
    /// Returns a reference that must be used within the read lock scope.
    pub fn with_metrics<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&EngineMetrics) -> R,
    {
        let engine = self.inner.read().unwrap();
        f(engine.metrics())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn temp_config() -> Config {
        let dir = tempfile::tempdir().unwrap();
        Config {
            data_dir: dir.path().to_path_buf(),
            memtable_max_size: 64 * 1024,
            sync_writes: true,
        }
    }

    #[test]
    fn test_concurrent_put_get() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();

        engine.put(b"test".to_vec(), b"value".to_vec()).unwrap();
        assert_eq!(engine.get(b"test"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_clone_and_share() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();

        let engine_clone = engine.clone();
        engine_clone.put(b"shared".to_vec(), b"data".to_vec()).unwrap();

        // Original engine sees the update
        assert_eq!(engine.get(b"shared"), Some(b"data".to_vec()));
    }

    #[test]
    fn test_multiple_concurrent_reads() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();
        engine.put(b"key".to_vec(), b"value".to_vec()).unwrap();

        let mut handles = vec![];

        // Spawn 10 concurrent readers
        for _ in 0..10 {
            let engine_clone = engine.clone();
            let handle = thread::spawn(move || {
                assert_eq!(engine_clone.get(b"key"), Some(b"value".to_vec()));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writers() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();
        let mut handles = vec![];

        // Spawn 5 concurrent writers
        for i in 0..5 {
            let engine_clone = engine.clone();
            let handle = thread::spawn(move || {
                let key = format!("key_{}", i).into_bytes();
                let value = format!("value_{}", i).into_bytes();
                engine_clone.put(key, value).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all writes succeeded
        assert_eq!(engine.len(), 5);
    }

    #[test]
    fn test_concurrent_read_write() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();
        engine.put(b"initial".to_vec(), b"value".to_vec()).unwrap();

        let mut handles = vec![];

        // 5 readers
        for _ in 0..5 {
            let engine_clone = engine.clone();
            let handle = thread::spawn(move || {
                engine_clone.get(b"initial");
            });
            handles.push(handle);
        }

        // 5 writers
        for i in 0..5 {
            let engine_clone = engine.clone();
            let handle = thread::spawn(move || {
                let key = format!("writer_{}", i).into_bytes();
                engine_clone.put(key, b"data".to_vec()).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(engine.len() >= 5); // At least the 5 writer keys
    }

    #[test]
    fn test_metrics_access() {
        let engine = ConcurrentOblivion::open(temp_config()).unwrap();
        engine.put(b"test".to_vec(), b"value".to_vec()).unwrap();

        engine.with_metrics(|metrics| {
            assert!(metrics.total_ops() > 0);
        });
    }
}
