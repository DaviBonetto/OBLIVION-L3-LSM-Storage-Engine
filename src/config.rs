//! OBLIVION - Engine Configuration
//! Defines tunable parameters for the LSM storage engine.

use std::path::PathBuf;

/// Configuration for the Oblivion storage engine.
#[derive(Debug, Clone)]
pub struct Config {
    /// Base directory for all data files (WAL, SSTables).
    pub data_dir: PathBuf,

    /// Maximum size of the MemTable in bytes before triggering a flush.
    pub memtable_max_size: usize,

    /// Whether to sync WAL writes to disk immediately (fsync).
    pub sync_writes: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            memtable_max_size: 4 * 1024 * 1024, // 4 MB
            sync_writes: true,
        }
    }
}

impl Config {
    /// Create a new Config with a custom data directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            ..Default::default()
        }
    }

    /// Set the maximum MemTable size before flush.
    pub fn with_memtable_max_size(mut self, size: usize) -> Self {
        self.memtable_max_size = size;
        self
    }

    /// Ensure the data directory exists.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }
}
