//! OBLIVION - Write-Ahead Log (WAL)
//! Provides durability by logging all mutations to disk
//! before they are applied to the in-memory MemTable.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::error::{OblivionError, Result};
use crate::types::{Key, Value};

/// Operation type for WAL entries.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum OpType {
    Put = 1,
    Delete = 2,
}

/// Write-Ahead Log for crash recovery and durability.
/// Every write operation is first appended to the WAL on disk
/// before being applied to the MemTable in memory.
///
/// ## Binary Format (per entry)
/// ```text
/// [op_type: 1 byte][key_len: 4 bytes (LE)][key: N bytes][val_len: 4 bytes (LE)][value: M bytes][crc: 4 bytes]
/// ```
pub struct WriteAheadLog {
    /// Path to the WAL file on disk.
    path: PathBuf,
    /// File handle opened for appending.
    file: File,
}

impl WriteAheadLog {
    /// Open or create a WAL file at the specified path.
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

    /// Encode a PUT entry into the binary WAL format.
    fn encode_put(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        // Op type
        buf.push(OpType::Put as u8);
        // Key length + key
        buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
        buf.extend_from_slice(key);
        // Value length + value
        buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buf.extend_from_slice(value);
        // CRC32 checksum
        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Encode a DELETE entry into the binary WAL format.
    fn encode_delete(key: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        // Op type
        buf.push(OpType::Delete as u8);
        // Key length + key
        buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
        buf.extend_from_slice(key);
        // Value length = 0 for deletes
        buf.extend_from_slice(&0u32.to_le_bytes());
        // CRC32 checksum
        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }
}
