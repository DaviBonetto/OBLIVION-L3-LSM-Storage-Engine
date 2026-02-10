//! OBLIVION - Write-Ahead Log (WAL)
//! Provides durability by logging all mutations to disk
//! before they are applied to the in-memory MemTable.

use std::path::PathBuf;

/// Write-Ahead Log for crash recovery and durability.
/// Every write operation is first appended to the WAL on disk
/// before being applied to the MemTable in memory.
pub struct WriteAheadLog {
    /// Path to the WAL file on disk.
    path: PathBuf,
}

impl WriteAheadLog {
    /// Create a new WAL instance with the given path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
