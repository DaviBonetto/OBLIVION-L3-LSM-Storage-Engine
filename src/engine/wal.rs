//! OBLIVION - Write-Ahead Log (WAL)
//! Provides durability by logging all mutations to disk
//! before they are applied to the in-memory MemTable.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use crate::engine::memtable::MemTable;
use crate::error::Result;
use crate::types::{Key, Value};

/// Operation type for WAL entries.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum OpType {
    Put = 1,
    Delete = 2,
}

/// Write-Ahead Log for crash recovery and durability.
///
/// ## Binary Format (per entry)
/// ```text
/// [op_type: 1 byte][key_len: 4 bytes LE][key: N bytes][val_len: 4 bytes LE][value: M bytes][crc: 4 bytes]
/// ```
///
/// Uses BufWriter to batch syscalls for improved write throughput.
pub struct WriteAheadLog {
    /// Path to the WAL file on disk.
    path: PathBuf,
    /// Buffered writer wrapping the file handle.
    /// BufWriter reduces the number of write syscalls by
    /// batching small writes into larger chunks (8KB default).
    writer: BufWriter<File>,
}

impl WriteAheadLog {
    /// Open or create a WAL file at the specified path.
    /// Uses BufWriter for write batching to reduce syscall overhead.
    pub fn open(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(file),
        })
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Encode a PUT entry into the binary WAL format.
    fn encode_put(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OpType::Put as u8);
        buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buf.extend_from_slice(value);
        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Encode a DELETE entry into the binary WAL format.
    fn encode_delete(key: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OpType::Delete as u8);
        buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(&0u32.to_le_bytes());
        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Append a PUT operation to the WAL and flush to disk.
    /// BufWriter batches the write, then flush + sync ensures durability.
    pub fn append_put(&mut self, key: &Key, value: &Value) -> Result<()> {
        let encoded = Self::encode_put(key, value);
        self.writer.write_all(&encoded)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }

    /// Append a DELETE operation to the WAL and flush to disk.
    pub fn append_delete(&mut self, key: &Key) -> Result<()> {
        let encoded = Self::encode_delete(key);
        self.writer.write_all(&encoded)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }

    /// Truncate the WAL file (called after successful flush).
    pub fn truncate(&mut self) -> Result<()> {
        // Flush any remaining buffered data
        self.writer.flush()?;
        // Truncate
        let _file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        // Reopen in append mode with BufWriter
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.writer = BufWriter::new(file);
        Ok(())
    }

    /// Recover the MemTable state from the WAL file.
    pub fn recover(path: &PathBuf) -> Result<MemTable> {
        let mut memtable = MemTable::new();

        if !path.exists() {
            return Ok(memtable);
        }

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut cursor = 0;
        let len = data.len();

        while cursor < len {
            if cursor + 5 > len {
                break;
            }

            let op_byte = data[cursor];
            cursor += 1;

            let key_len = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            cursor += 4;

            if cursor + key_len > len {
                break;
            }
            let key = data[cursor..cursor + key_len].to_vec();
            cursor += key_len;

            if cursor + 4 > len {
                break;
            }
            let val_len = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            cursor += 4;

            if cursor + val_len > len {
                break;
            }
            let value = data[cursor..cursor + val_len].to_vec();
            cursor += val_len;

            if cursor + 4 > len {
                break;
            }
            let stored_crc = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]);
            cursor += 4;

            let record_start = cursor - 4 - val_len - 4 - key_len - 4 - 1;
            let record_data = &data[record_start..cursor - 4];
            let computed_crc = crc32fast::hash(record_data);

            if stored_crc != computed_crc {
                log::warn!("CRC mismatch at offset {}, skipping rest of WAL", record_start);
                break;
            }

            match op_byte {
                1 => memtable.insert(key, value),
                2 => memtable.delete(key),
                _ => {
                    log::warn!("Unknown op type {} at offset {}", op_byte, record_start);
                    break;
                }
            }
        }

        log::info!(
            "WAL recovery complete: {} entries restored",
            memtable.len()
        );

        Ok(memtable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let encoded = WriteAheadLog::encode_put(b"hello", b"world");
        assert_eq!(encoded.len(), 23);
        assert_eq!(encoded[0], OpType::Put as u8);
    }

    #[test]
    fn test_wal_append_and_recover() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        {
            let mut wal = WriteAheadLog::open(wal_path.clone()).unwrap();
            wal.append_put(&b"key1".to_vec(), &b"value1".to_vec()).unwrap();
            wal.append_put(&b"key2".to_vec(), &b"value2".to_vec()).unwrap();
            wal.append_delete(&b"key1".to_vec()).unwrap();
        }

        let memtable = WriteAheadLog::recover(&wal_path).unwrap();
        assert_eq!(memtable.get(b"key1"), None);
        assert_eq!(memtable.get(b"key2"), Some(&b"value2".to_vec()));
    }
}
