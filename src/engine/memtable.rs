//! OBLIVION - MemTable (In-Memory Sorted Map)
//! The MemTable is the write-buffer of the LSM-Tree.
//! All writes go here first before being flushed to SSTables on disk.

use std::collections::BTreeMap;

use crate::types::{Key, Value};

/// In-memory sorted key-value store backed by a BTreeMap.
/// Serves as the write buffer in the LSM-Tree architecture.
pub struct MemTable {
    /// Sorted map storing key-value pairs.
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
}
