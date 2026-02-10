//! OBLIVION - Storage Engine Module
//! Top-level module for the LSM-Tree storage engine components.

pub mod memtable;
pub mod wal;

use crate::config::Config;
use crate::error::Result;
use crate::types::{Key, Value};

use self::memtable::MemTable;
use self::wal::WriteAheadLog;

/// The core Oblivion storage engine.
/// Coordinates the MemTable, WAL, and (future) SSTables
/// to provide a durable key-value store based on LSM-Tree architecture.
pub struct Oblivion {
    /// In-memory sorted buffer for recent writes.
    memtable: MemTable,
    /// Write-ahead log for crash recovery.
    wal: WriteAheadLog,
    /// Engine configuration.
    config: Config,
}

impl Oblivion {
    /// Open or create an Oblivion storage engine at the configured path.
    pub fn open(config: Config) -> Result<Self> {
        config.ensure_dirs()?;

        let wal_path = config.data_dir.join("oblivion.wal");
        let memtable = WriteAheadLog::recover(&wal_path)?;
        let wal = WriteAheadLog::open(wal_path)?;

        log::info!(
            "Oblivion engine opened at {:?} ({} entries recovered)",
            config.data_dir,
            memtable.len()
        );

        Ok(Self {
            memtable,
            wal,
            config,
        })
    }

    /// Insert a key-value pair into the storage engine.
    /// The write path: WAL (disk) -> MemTable (memory).
    /// This ensures durability: if the process crashes after
    /// WAL write but before MemTable update, the WAL recovery
    /// will replay the operation on next startup.
    pub fn put(&mut self, key: Key, value: Value) -> Result<()> {
        // Step 1: Write to WAL first (durability)
        self.wal.append_put(&key, &value)?;
        // Step 2: Write to MemTable (fast reads)
        self.memtable.insert(key, value);
        Ok(())
    }

    /// Delete a key from the storage engine.
    /// Writes a tombstone to both WAL and MemTable.
    pub fn delete(&mut self, key: Key) -> Result<()> {
        self.wal.append_delete(&key)?;
        self.memtable.delete(key);
        Ok(())
    }
}
