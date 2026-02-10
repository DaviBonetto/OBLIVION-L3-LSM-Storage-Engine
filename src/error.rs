//! OBLIVION - Custom Error Types
//! Defines the error hierarchy for the LSM storage engine.

use thiserror::Error;

/// Custom Result type for the Oblivion engine.
pub type Result<T> = std::result::Result<T, OblivionError>;

/// Error types for the Oblivion storage engine.
#[derive(Error, Debug)]
pub enum OblivionError {
    /// I/O errors from file operations (WAL, SSTable).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization errors.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Data corruption detected (CRC mismatch).
    #[error("Data corruption detected: {0}")]
    Corruption(String),

    /// Key not found in the storage engine.
    #[error("Key not found")]
    KeyNotFound,

    /// WAL recovery failure.
    #[error("WAL recovery failed: {0}")]
    RecoveryFailed(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}
