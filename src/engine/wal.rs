//! OBLIVION - Write-Ahead Log (WAL)
//! Provides durability by logging all mutations to disk
//! before they are applied to the in-memory MemTable.

use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use crate::error::Result;

/// Write-Ahead Log for crash recovery and durability.
/// Every write operation is first appended to the WAL on disk
/// before being applied to the MemTable in memory.
pub struct WriteAheadLog {
    /// Path to the WAL file on disk.
    path: PathBuf,
    /// File handle opened for appending.
    file: File,
}

impl WriteAheadLog {
    /// Open or create a WAL file at the specified path.
    /// The file is opened in append mode for sequential writes.
    pub fn open(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self { path, file })
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns a reference to the underlying file.
    pub fn file(&self) -> &File {
        &self.file
    }
}
