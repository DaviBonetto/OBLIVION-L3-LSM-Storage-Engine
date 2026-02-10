//! OBLIVION - Storage Engine Module
//! Top-level module for the LSM-Tree storage engine components.

pub mod bloom;
pub mod memtable;
pub mod metrics;
pub mod sstable;
pub mod wal;

use crate::config::Config;
use crate::error::Result;
use crate::types::{Key, Value};

use self::memtable::MemTable;
use self::metrics::EngineMetrics;
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
    /// Counter for SSTable file naming.
    flush_count: u64,
    /// Runtime operation metrics.
    metrics: EngineMetrics,
}

impl Oblivion {
    /// Open or create an Oblivion storage engine at the configured path.
    pub fn open(config: Config) -> Result<Self> {
        config.ensure_dirs()?;

        let wal_path = config.data_dir.join("oblivion.wal");
        let memtable = WriteAheadLog::recover(&wal_path)?;
        let wal = WriteAheadLog::open(wal_path)?;

        let metrics = EngineMetrics::new();
        if !memtable.is_empty() {
            metrics.record_recovery();
        }

        log::info!(
            "Oblivion engine opened at {:?} ({} entries recovered)",
            config.data_dir,
            memtable.len()
        );

        Ok(Self {
            memtable,
            wal,
            config,
            flush_count: 0,
            metrics,
        })
    }

    /// Insert a key-value pair into the storage engine.
    /// Write path: WAL (disk) -> MemTable (memory) -> check flush.
    pub fn put(&mut self, key: Key, value: Value) -> Result<()> {
        self.metrics.record_put(key.len(), value.len());
        self.wal.append_put(&key, &value)?;
        self.memtable.insert(key, value);

        // Check if MemTable needs flushing
        self.maybe_flush()?;

        Ok(())
    }

    /// Get a value by key from the storage engine.
    /// Read path: MemTable (memory) -> (future: SSTables on disk).
    pub fn get(&self, key: &[u8]) -> Option<Value> {
        let result = self.memtable.get(key).cloned();
        self.metrics.record_get(result.as_ref().map(|v| v.len()));
        result
    }

    /// Delete a key from the storage engine.
    pub fn delete(&mut self, key: Key) -> Result<()> {
        self.metrics.record_delete();
        self.wal.append_delete(&key)?;
        self.memtable.delete(key);
        self.maybe_flush()?;
        Ok(())
    }

    /// Scan all key-value pairs in sorted order.
    pub fn scan(&self) -> Vec<(Key, Value)> {
        self.metrics.record_scan();
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

    /// Returns a reference to the engine metrics.
    pub fn metrics(&self) -> &EngineMetrics {
        &self.metrics
    }

    /// Check if the MemTable exceeds the configured size threshold.
    /// If so, trigger a flush: simulate writing to SSTable,
    /// truncate the WAL, and reset the MemTable.
    fn maybe_flush(&mut self) -> Result<()> {
        if self.memtable.size() >= self.config.memtable_max_size {
            log::info!(
                "MemTable size ({} bytes) exceeds threshold ({} bytes), triggering flush...",
                self.memtable.size(),
                self.config.memtable_max_size
            );

            // In production: write MemTable entries to SSTable
            let sstable_path = self.config.data_dir.join(format!(
                "sstable_{:06}.sst",
                self.flush_count
            ));
            let entries = self.scan();
            let _sstable = sstable::SSTable::flush_from_memtable(sstable_path, &entries)?;

            // Truncate WAL (data is now in SSTable)
            self.wal.truncate()?;

            // Reset MemTable
            self.memtable.clear();
            self.flush_count += 1;
            self.metrics.record_flush();

            log::info!(
                "Flush #{} complete. {} entries written to SSTable.",
                self.flush_count,
                entries.len()
            );
        }

        Ok(())
    }
}
