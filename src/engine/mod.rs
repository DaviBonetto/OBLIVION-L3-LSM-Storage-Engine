//! OBLIVION - Storage Engine Module
//! Top-level module for the LSM-Tree storage engine components.

pub mod memtable;
pub mod wal;

use crate::config::Config;
use crate::error::Result;

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
    /// If a WAL file exists, it will be replayed to recover state.
    pub fn open(config: Config) -> Result<Self> {
        config.ensure_dirs()?;

        let wal_path = config.data_dir.join("oblivion.wal");

        // Recover MemTable from WAL if it exists
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
}
