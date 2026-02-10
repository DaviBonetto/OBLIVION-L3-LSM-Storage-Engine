//! OBLIVION - MemTable (In-Memory Sorted Map)
//! The MemTable is the write-buffer of the LSM-Tree.
//! All writes go here first before being flushed to SSTables on disk.

use std::collections::BTreeMap;

use crate::types::{Key, Value};

/// In-memory sorted key-value store backed by a BTreeMap.
/// Serves as the write buffer in the LSM-Tree architecture.
pub struct MemTable {
    /// Sorted map storing key-value pairs.
    /// A `None` value represents a tombstone (deletion marker).
    entries: BTreeMap<Key, Option<Value>>,
    /// Current approximate size in bytes.
    size_bytes: usize,
}

impl MemTable {
    /// Create a new, empty MemTable.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            size_bytes: 0,
        }
    }

    /// Returns the approximate size of the MemTable in bytes.
    pub fn size(&self) -> usize {
        self.size_bytes
    }

    /// Returns the number of entries in the MemTable.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the MemTable is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a key-value pair into the MemTable.
    /// If the key already exists, the old value is replaced.
    pub fn insert(&mut self, key: Key, value: Value) {
        let entry_size = key.len() + value.len();
        if let Some(old_val) = self.entries.get(&key) {
            let old_size = key.len() + old_val.as_ref().map_or(0, |v| v.len());
            self.size_bytes = self.size_bytes.saturating_sub(old_size);
        }
        self.size_bytes += entry_size;
        self.entries.insert(key, Some(value));
    }

    /// Get a value by key from the MemTable.
    /// Returns `None` if the key does not exist or has been deleted (tombstone).
    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        match self.entries.get(key) {
            Some(Some(value)) => Some(value),
            Some(None) => None,
            None => None,
        }
    }

    /// Check if a key exists in the MemTable (including tombstones).
    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.entries.contains_key(key)
    }

    /// Delete a key by inserting a tombstone marker.
    pub fn delete(&mut self, key: Key) {
        let key_size = key.len();
        if let Some(old_val) = self.entries.get(&key) {
            let old_size = key.len() + old_val.as_ref().map_or(0, |v| v.len());
            self.size_bytes = self.size_bytes.saturating_sub(old_size);
        }
        self.size_bytes += key_size;
        self.entries.insert(key, None);
    }

    /// Clear all entries from the MemTable and reset size.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.size_bytes = 0;
    }

    /// Returns a reference to the inner BTreeMap for iteration.
    pub fn entries(&self) -> &BTreeMap<Key, Option<Value>> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut table = MemTable::new();
        table.insert(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(table.get(b"key1"), Some(&b"value1".to_vec()));
    }

    #[test]
    fn test_get_nonexistent() {
        let table = MemTable::new();
        assert_eq!(table.get(b"missing"), None);
    }

    #[test]
    fn test_overwrite() {
        let mut table = MemTable::new();
        table.insert(b"key".to_vec(), b"old".to_vec());
        table.insert(b"key".to_vec(), b"new".to_vec());
        assert_eq!(table.get(b"key"), Some(&b"new".to_vec()));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_delete_tombstone() {
        let mut table = MemTable::new();
        table.insert(b"key".to_vec(), b"value".to_vec());
        table.delete(b"key".to_vec());
        assert_eq!(table.get(b"key"), None);
        assert!(table.contains_key(b"key")); // tombstone still exists
    }

    #[test]
    fn test_size_tracking() {
        let mut table = MemTable::new();
        assert_eq!(table.size(), 0);
        table.insert(b"abc".to_vec(), b"12345".to_vec()); // 3 + 5 = 8
        assert_eq!(table.size(), 8);
    }

    #[test]
    fn test_clear() {
        let mut table = MemTable::new();
        table.insert(b"k1".to_vec(), b"v1".to_vec());
        table.insert(b"k2".to_vec(), b"v2".to_vec());
        table.clear();
        assert!(table.is_empty());
        assert_eq!(table.size(), 0);
    }
}
