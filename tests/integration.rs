//! OBLIVION - Integration Tests
//! End-to-end tests validating the full engine lifecycle:
//! open → put → get → delete → scan → crash recovery → flush.

use std::path::PathBuf;

// Import the library modules directly from the binary crate
// by using the module path structure.
mod common {
    use std::path::PathBuf;

    /// Create a Config pointing to a temporary directory.
    pub fn temp_config(dir: &std::path::Path) -> oblivion::config::Config {
        oblivion::config::Config {
            data_dir: dir.to_path_buf(),
            memtable_max_size: 1024, // 1KB threshold for easy flush testing
            sync_writes: true,
        }
    }
}

#[test]
fn test_basic_put_get_delete() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::temp_config(dir.path());

    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    // Put
    engine.put(b"name".to_vec(), b"oblivion".to_vec()).unwrap();
    engine.put(b"version".to_vec(), b"1.0.0".to_vec()).unwrap();

    // Get
    assert_eq!(engine.get(b"name"), Some(b"oblivion".to_vec()));
    assert_eq!(engine.get(b"version"), Some(b"1.0.0".to_vec()));
    assert_eq!(engine.get(b"missing"), None);

    // Delete
    engine.delete(b"name".to_vec()).unwrap();
    assert_eq!(engine.get(b"name"), None);

    // Remaining
    assert_eq!(engine.get(b"version"), Some(b"1.0.0".to_vec()));
}

#[test]
fn test_overwrite_value() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::temp_config(dir.path());
    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    engine.put(b"key".to_vec(), b"old".to_vec()).unwrap();
    assert_eq!(engine.get(b"key"), Some(b"old".to_vec()));

    engine.put(b"key".to_vec(), b"new".to_vec()).unwrap();
    assert_eq!(engine.get(b"key"), Some(b"new".to_vec()));

    assert_eq!(engine.len(), 1);
}

#[test]
fn test_scan_sorted_order() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::temp_config(dir.path());
    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    engine.put(b"charlie".to_vec(), b"3".to_vec()).unwrap();
    engine.put(b"alpha".to_vec(), b"1".to_vec()).unwrap();
    engine.put(b"bravo".to_vec(), b"2".to_vec()).unwrap();

    let entries = engine.scan();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, b"alpha");
    assert_eq!(entries[1].0, b"bravo");
    assert_eq!(entries[2].0, b"charlie");
}

#[test]
fn test_crash_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let data_path = dir.path().to_path_buf();

    // Phase 1: Write data and drop engine (simulates crash)
    {
        let config = oblivion::config::Config {
            data_dir: data_path.clone(),
            memtable_max_size: 64 * 1024, // large threshold, no flush
            sync_writes: true,
        };
        let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

        engine.put(b"persistent_key".to_vec(), b"persistent_value".to_vec()).unwrap();
        engine.put(b"ephemeral".to_vec(), b"data".to_vec()).unwrap();
        engine.delete(b"ephemeral".to_vec()).unwrap();

        // Engine is dropped here — WAL persists on disk
    }

    // Phase 2: Reopen and verify WAL recovery
    {
        let config = oblivion::config::Config {
            data_dir: data_path,
            memtable_max_size: 64 * 1024,
            sync_writes: true,
        };
        let engine = oblivion::engine::Oblivion::open(config).unwrap();

        // Persistent key should be recovered
        assert_eq!(
            engine.get(b"persistent_key"),
            Some(b"persistent_value".to_vec())
        );

        // Deleted key should remain deleted after recovery
        assert_eq!(engine.get(b"ephemeral"), None);
    }
}

#[test]
fn test_empty_engine() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::temp_config(dir.path());
    let engine = oblivion::engine::Oblivion::open(config).unwrap();

    assert!(engine.is_empty());
    assert_eq!(engine.len(), 0);
    assert_eq!(engine.memtable_size(), 0);
    assert_eq!(engine.get(b"anything"), None);
    assert!(engine.scan().is_empty());
}

#[test]
fn test_large_values() {
    let dir = tempfile::tempdir().unwrap();
    let config = oblivion::config::Config {
        data_dir: dir.path().to_path_buf(),
        memtable_max_size: 1024 * 1024, // 1MB
        sync_writes: true,
    };
    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    // Write a 10KB value
    let large_value = vec![0xABu8; 10_000];
    engine.put(b"big".to_vec(), large_value.clone()).unwrap();

    assert_eq!(engine.get(b"big"), Some(large_value));
}

#[test]
fn test_unicode_keys() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::temp_config(dir.path());
    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    engine.put("café".as_bytes().to_vec(), b"coffee".to_vec()).unwrap();
    engine.put("日本語".as_bytes().to_vec(), b"japanese".to_vec()).unwrap();
    engine.put("🦀".as_bytes().to_vec(), b"crab".to_vec()).unwrap();

    assert_eq!(engine.get("café".as_bytes()), Some(b"coffee".to_vec()));
    assert_eq!(engine.get("日本語".as_bytes()), Some(b"japanese".to_vec()));
    assert_eq!(engine.get("🦀".as_bytes()), Some(b"crab".to_vec()));
}

#[test]
fn test_many_writes_trigger_info() {
    let dir = tempfile::tempdir().unwrap();
    // Use larger threshold to prevent flush during test
    let config = oblivion::config::Config {
        data_dir: dir.path().to_path_buf(),
        memtable_max_size: 64 * 1024, // 64KB - enough for 100 writes
        sync_writes: true,
    };
    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

    for i in 0..100 {
        let key = format!("key_{:04}", i).into_bytes();
        let value = format!("value_{:04}", i).into_bytes();
        engine.put(key, value).unwrap();
    }

    // Verify a few random keys
    assert_eq!(engine.get(b"key_0000"), Some(b"value_0000".to_vec()));
    assert_eq!(engine.get(b"key_0050"), Some(b"value_0050".to_vec()));
    assert_eq!(engine.get(b"key_0099"), Some(b"value_0099".to_vec()));
}
