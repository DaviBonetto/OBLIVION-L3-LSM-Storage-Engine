//! OBLIVION - Core Type Definitions
//! Defines fundamental types used across the storage engine.

/// Key type for the storage engine.
/// Using Vec<u8> allows arbitrary binary keys.
pub type Key = Vec<u8>;

/// Value type for the storage engine.
/// Using Vec<u8> allows arbitrary binary values.
pub type Value = Vec<u8>;

/// Represents a single entry in the storage engine.
/// A `None` value indicates a tombstone (deletion marker).
#[derive(Debug, Clone)]
pub struct Entry {
    pub key: Key,
    pub value: Option<Value>,
    pub timestamp: u64,
}

impl Entry {
    /// Create a new entry with a value (PUT operation).
    pub fn put(key: Key, value: Value) -> Self {
        Self {
            key,
            value: Some(value),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        }
    }

    /// Create a tombstone entry (DELETE operation).
    pub fn delete(key: Key) -> Self {
        Self {
            key,
            value: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        }
    }

    /// Returns true if this entry is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        self.value.is_none()
    }
}
