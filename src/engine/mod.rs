//! OBLIVION - Storage Engine Module
//! Top-level module for the LSM-Tree storage engine components.

pub mod memtable;
pub mod sstable;
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
    /// Write path: WAL (disk) -> MemTable (memory).
    pub fn put(&mut self, key: Key, value: Value) -> Result<()> {
        self.wal.append_put(&key, &value)?;
        self.memtable.insert(key, value);
        Ok(())
    }

    /// Get a value by key from the storage engine.
    /// Read path: MemTable (memory) -> (future: SSTables on disk).
    /// In a full LSM implementation, if not found in MemTable,
    /// we would check immutable MemTables, then SSTables (L0 -> LN).
    pub fn get(&self, key: &[u8]) -> Option<Value> {
        self.memtable.get(key).cloned()
    }

    /// Delete a key from the storage engine.
    pub fn delete(&mut self, key: Key) -> Result<()> {
        self.wal.append_delete(&key)?;
        self.memtable.delete(key);
        Ok(())
    }

    /// Scan all key-value pairs in sorted order.
    pub fn scan(&self) -> Vec<(Key, Value)> {
        self.memtable
            .scan()
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Returns the number of entries in the MemTable.
    pub fn len(&self) -> usize {
        self.memtable.len()
    }

    /// Returns true if the engine has no entries.
    pub fn is_empty(&self) -> bool {
        self.memtable.is_empty()
    }

    /// Returns the approximate size of the MemTable in bytes.
    pub fn memtable_size(&self) -> usize {
        self.memtable.size()
    }
}
