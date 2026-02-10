//! OBLIVION - SSTable (Sorted String Table)
//! Immutable on-disk data structure for persisting flushed MemTable data.
//! This is a stub/placeholder for the full SSTable implementation.

use std::path::PathBuf;

use crate::types::{Key, Value};

/// Sorted String Table - immutable on-disk storage.
/// In a full LSM implementation, SSTables are created when the
/// MemTable exceeds its size threshold and needs to be flushed.
///
/// ## Future Implementation
/// - Block-based format with index
/// - Bloom filter for fast negative lookups
/// - Compression (LZ4/Snappy)
/// - Multi-level compaction (L0 -> L1 -> ... -> LN)
pub struct SSTable {
    /// Path to the SSTable file on disk.
    path: PathBuf,
    /// Number of entries in this SSTable.
    entry_count: usize,
    /// Size of the SSTable file in bytes.
    file_size: u64,
}

impl SSTable {
    /// Create a new SSTable reference (stub).
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entry_count: 0,
            file_size: 0,
        }
    }

    /// Returns the path to the SSTable file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the number of entries.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Returns the file size in bytes.
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Flush a MemTable's entries to disk as an SSTable (stub).
    /// In production, this would write a block-based format
    /// with an index and optional bloom filter.
    pub fn flush_from_memtable(
        _path: PathBuf,
        _entries: &[(Key, Value)],
    ) -> crate::error::Result<Self> {
        // TODO: Implement actual SSTable flush
        // For now, this is a mock that simulates the flush
        log::info!(
            "SSTable flush triggered (stub) - {} entries",
            _entries.len()
        );
        Ok(Self {
            path: _path,
            entry_count: _entries.len(),
            file_size: 0,
        })
    }
}
