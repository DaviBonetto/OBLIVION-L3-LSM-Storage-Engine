//! OBLIVION - Compaction Strategy
//! Implements size-tiered compaction to merge overlapping SSTables
//! and reclaim space from tombstoned deletions.
//!
//! ## LSM-Tree Compaction
//! As MemTable flushes accumulate, SSTables with overlapping key ranges
//! need to be merged to maintain read performance and reduce disk usage.
//!
//! ## Size-Tiered Strategy
//! - Group SSTables by size tier (e.g., 4MB, 40MB, 400MB)
//! - When N tables accumulate in a tier, merge them into the next tier
//! - Simpler than leveled compaction, good for write-heavy workloads

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::types::{Key, Value};

/// Trait defining a compaction strategy.
pub trait CompactionStrategy {
    /// Select which SSTables should be compacted together.
    /// Returns a list of SSTable IDs to merge.
    fn select_compaction(&self, sstables: &[SStableInfo]) -> Option<Vec<usize>>;

    /// Returns the human-readable name of this strategy.
    fn name(&self) -> &str;
}

/// Metadata about an SSTable file.
#[derive(Debug, Clone)]
pub struct SStableInfo {
    /// Unique SSTable ID.
    pub id: usize,
    /// Path to the SSTable file.
    pub path: PathBuf,
    /// Approximate size in bytes.
    pub size: usize,
    /// Smallest key in this SSTable.
    pub min_key: Key,
    /// Largest key in this SSTable.
    pub max_key: Key,
}

impl SStableInfo {
    /// Check if two SStables have overlapping key ranges.
    pub fn overlaps(&self, other: &SStableInfo) -> bool {
        self.min_key <= other.max_key && self.max_key >= other.min_key
    }
}

/// Size-tiered compaction strategy.
///
/// ## Algorithm
/// - Define size tiers: T0 (0-4MB), T1 (4-40MB), T2 (40-400MB), ...
/// - When a tier has >= `threshold` tables, compact them into next tier
/// - Compaction merges overlapping ranges and removes tombstones
///
/// ## Example
/// ```
/// // T0: [1MB, 2MB, 3MB, 1MB] (4 tables >= threshold 4)
/// // => Compact into T1: [7MB]
/// ```
pub struct SizeTieredCompaction {
    /// Number of tables per tier before triggering compaction.
    threshold: usize,
    /// Size multiplier between tiers (default: 10x).
    size_ratio: usize,
}

impl SizeTieredCompaction {
    /// Create a new size-tiered compaction strategy.
    ///
    /// # Arguments
    /// * `threshold` - Number of tables to accumulate before compacting (typically 4)
    /// * `size_ratio` - Multiplier between tiers (typically 10)
    pub fn new(threshold: usize, size_ratio: usize) -> Self {
        Self {
            threshold,
            size_ratio,
        }
    }

    /// Get the tier level for a given SSTable size.
    fn tier_for_size(&self, size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        let base = self.size_ratio;
        let mut tier = 0;
        let mut upper_bound = 4 * 1024 * 1024; // 4MB base

        while size > upper_bound {
            tier += 1;
            upper_bound *= base;
        }
        tier
    }
}

impl CompactionStrategy for SizeTieredCompaction {
    fn select_compaction(&self, sstables: &[SStableInfo]) -> Option<Vec<usize>> {
        if sstables.is_empty() {
            return None;
        }

        // Group SSTables by tier
        let mut tiers: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (idx, table) in sstables.iter().enumerate() {
            let tier = self.tier_for_size(table.size);
            tiers.entry(tier).or_default().push(idx);
        }

        // Find the first tier with >= threshold tables
        for (_tier, table_ids) in tiers.iter() {
            if table_ids.len() >= self.threshold {
                return Some(table_ids.clone());
            }
        }

        None
    }

    fn name(&self) -> &str {
        "SizeTieredCompaction"
    }
}

/// Merge multiple SSTables into a single compacted SSTable.
///
/// ## Algorithm
/// 1. Read all entries from input SSTables
/// 2. Merge into a sorted BTreeMap (later keys override earlier)
/// 3. Remove tombstones (deleted keys)
/// 4. Write merged entries to new SSTable
///
/// ## Returns
/// A vector of (key, value) pairs representing the compacted data.
pub fn compact_sstables(sstables: Vec<Vec<(Key, Value)>>) -> Vec<(Key, Value)> {
    let mut merged = BTreeMap::new();

    // Merge all entries (later SSTables override earlier)
    for sstable in sstables {
        for (key, value) in sstable {
            merged.insert(key, value);
        }
    }

    // Filter out tombstones (empty values indicate deletion)
    merged.into_iter().filter(|(_k, v)| !v.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sstable_overlap() {
        let s1 = SStableInfo {
            id: 0,
            path: PathBuf::from("s1.sst"),
            size: 1000,
            min_key: b"a".to_vec(),
            max_key: b"m".to_vec(),
        };

        let s2 = SStableInfo {
            id: 1,
            path: PathBuf::from("s2.sst"),
            size: 1000,
            min_key: b"k".to_vec(),
            max_key: b"z".to_vec(),
        };

        let s3 = SStableInfo {
            id: 2,
            path: PathBuf::from("s3.sst"),
            size: 1000,
            min_key: b"n".to_vec(),
            max_key: b"z".to_vec(),
        };

        assert!(s1.overlaps(&s2)); // a..m overlaps k..z
        assert!(!s1.overlaps(&s3)); // a..m doesn't overlap n..z
        assert!(s2.overlaps(&s3)); // k..z overlaps n..z
    }

    #[test]
    fn test_tier_calculation() {
        let strategy = SizeTieredCompaction::new(4, 10);

        assert_eq!(strategy.tier_for_size(1024 * 1024), 0); // 1MB → T0
        assert_eq!(strategy.tier_for_size(4 * 1024 * 1024), 0); // 4MB → T0
        assert_eq!(strategy.tier_for_size(10 * 1024 * 1024), 1); // 10MB → T1
        assert_eq!(strategy.tier_for_size(40 * 1024 * 1024), 1); // 40MB → T1
        assert_eq!(strategy.tier_for_size(100 * 1024 * 1024), 2); // 100MB → T2
    }

    #[test]
    fn test_select_compaction_below_threshold() {
        let strategy = SizeTieredCompaction::new(4, 10);

        let sstables = vec![
            SStableInfo {
                id: 0,
                path: PathBuf::from("0.sst"),
                size: 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
            SStableInfo {
                id: 1,
                path: PathBuf::from("1.sst"),
                size: 2 * 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
        ];

        // Only 2 tables, threshold is 4
        assert_eq!(strategy.select_compaction(&sstables), None);
    }

    #[test]
    fn test_select_compaction_trigger() {
        let strategy = SizeTieredCompaction::new(4, 10);

        let sstables = vec![
            SStableInfo {
                id: 0,
                path: PathBuf::from("0.sst"),
                size: 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
            SStableInfo {
                id: 1,
                path: PathBuf::from("1.sst"),
                size: 2 * 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
            SStableInfo {
                id: 2,
                path: PathBuf::from("2.sst"),
                size: 3 * 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
            SStableInfo {
                id: 3,
                path: PathBuf::from("3.sst"),
                size: 1024 * 1024,
                min_key: vec![],
                max_key: vec![],
            },
        ];

        // 4 tables in T0, should trigger compaction
        let result = strategy.select_compaction(&sstables);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 4);
    }

    #[test]
    fn test_compact_sstables_merge() {
        let sst1 = vec![
            (b"a".to_vec(), b"value1".to_vec()),
            (b"b".to_vec(), b"value2".to_vec()),
        ];

        let sst2 = vec![
            (b"a".to_vec(), b"new_value1".to_vec()), // overwrites
            (b"c".to_vec(), b"value3".to_vec()),
        ];

        let merged = compact_sstables(vec![sst1, sst2]);

        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0], (b"a".to_vec(), b"new_value1".to_vec()));
        assert_eq!(merged[1], (b"b".to_vec(), b"value2".to_vec()));
        assert_eq!(merged[2], (b"c".to_vec(), b"value3".to_vec()));
    }

    #[test]
    fn test_compact_sstables_tombstone_removal() {
        let sst1 = vec![
            (b"a".to_vec(), b"value1".to_vec()),
            (b"b".to_vec(), b"value2".to_vec()),
        ];

        let sst2 = vec![
            (b"a".to_vec(), b"".to_vec()), // tombstone
            (b"c".to_vec(), b"value3".to_vec()),
        ];

        let merged = compact_sstables(vec![sst1, sst2]);

        // 'a' should be filtered out (tombstone)
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], (b"b".to_vec(), b"value2".to_vec()));
        assert_eq!(merged[1], (b"c".to_vec(), b"value3".to_vec()));
    }
}
