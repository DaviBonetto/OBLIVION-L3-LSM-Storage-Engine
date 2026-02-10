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

impl Default for MemTable {
    fn default() -> Self {
        Self::new()
    }
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

    /// Scan all key-value pairs in sorted order.
    /// Tombstones (deleted keys) are excluded from the results.
    pub fn scan(&self) -> Vec<(&Key, &Value)> {
        self.entries
            .iter()
            .filter_map(|(k, v)| v.as_ref().map(|val| (k, val)))
            .collect()
    }

    /// Scan a range of keys [start, end) in sorted order.
    /// Tombstones are excluded from the results.
    pub fn scan_range(&self, start: &[u8], end: &[u8]) -> Vec<(&Key, &Value)> {
        use std::ops::Bound;
        self.entries
            .range::<Vec<u8>, _>((
                Bound::Included(start.to_vec()),
                Bound::Excluded(end.to_vec()),
            ))
            .filter_map(|(k, v)| v.as_ref().map(|val| (k, val)))
            .collect()
    }

    /// Scan keys with a given prefix in sorted order.
    pub fn scan_prefix(&self, prefix: &[u8]) -> Vec<(&Key, &Value)> {
        self.entries
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter_map(|(k, v)| v.as_ref().map(|val| (k, val)))
            .collect()
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
        assert!(table.contains_key(b"key"));
    }

    #[test]
    fn test_size_tracking() {
        let mut table = MemTable::new();
        assert_eq!(table.size(), 0);
        table.insert(b"abc".to_vec(), b"12345".to_vec());
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

    #[test]
    fn test_scan_sorted_order() {
        let mut table = MemTable::new();
        table.insert(b"charlie".to_vec(), b"3".to_vec());
        table.insert(b"alpha".to_vec(), b"1".to_vec());
        table.insert(b"bravo".to_vec(), b"2".to_vec());
        let results = table.scan();
        let keys: Vec<&[u8]> = results.iter().map(|(k, _)| k.as_slice()).collect();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], b"alpha");
        assert_eq!(keys[1], b"bravo");
        assert_eq!(keys[2], b"charlie");
    }

    #[test]
    fn test_scan_excludes_tombstones() {
        let mut table = MemTable::new();
        table.insert(b"a".to_vec(), b"1".to_vec());
        table.insert(b"b".to_vec(), b"2".to_vec());
        table.delete(b"a".to_vec());
        let results = table.scan();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, b"b");
    }

    #[test]
    fn test_scan_prefix() {
        let mut table = MemTable::new();
        table.insert(b"user:1".to_vec(), b"alice".to_vec());
        table.insert(b"user:2".to_vec(), b"bob".to_vec());
        table.insert(b"item:1".to_vec(), b"sword".to_vec());
        let results = table.scan_prefix(b"user:");
        assert_eq!(results.len(), 2);
    }
}
