//! OBLIVION - LSM-Tree Key-Value Storage Engine
//! 
//! A crash-recoverable storage engine based on the Log-Structured Merge-Tree (LSM-Tree) architecture.
//!
//! ## Features
//! - **Write-Ahead Log (WAL)**: Crash recovery with CRC32 integrity checks
//! - **MemTable**: In-memory BTreeMap for fast writes
//! - **SSTable**: Persistent on-disk sorted storage
//! - **Bloom Filter**: Probabilistic lookups for negative queries
//! - **TTL Support**: Redis-like key expiration
//! - **Metrics**: Lock-free atomic counters for observability
//! - **Compaction**: Size-tiered LSM compaction strategy
//! - **Concurrency**: Thread-safe Arc + RwLock wrapper
//!
//! ## Example
//! ```no_run
//! use oblivion::{config::Config, engine::Oblivion};
//!
//! let config = Config::default();
//! let mut engine = Oblivion::open(config).unwrap();
//!
//! engine.put(b"key".to_vec(), b"value".to_vec()).unwrap();
//! assert_eq!(engine.get(b"key"), Some(b"value".to_vec()));
//! ```

pub mod config;
pub mod engine;
pub mod error;
pub mod types;
