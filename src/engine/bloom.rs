//! OBLIVION - Bloom Filter
//! A space-efficient probabilistic data structure used to test
//! whether an element is a member of a set.
//!
//! False positives are possible, but false negatives are not.
//! Used in LSM-Trees to skip SSTable reads for keys that
//! definitely do not exist in a given table.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A Bloom filter for probabilistic set membership testing.
///
/// ## How it works
/// - Multiple hash functions map each key to bit positions
/// - On insert: set all corresponding bits to 1
/// - On lookup: check if all corresponding bits are 1
/// - If any bit is 0 → key is **definitely not** in the set
/// - If all bits are 1 → key is **probably** in the set
///
/// ## False Positive Rate
/// With `k` hash functions and `m` bits for `n` inserted elements:
/// `FPR ≈ (1 - e^(-kn/m))^k`
pub struct BloomFilter {
    /// Bit array stored as bytes.
    bits: Vec<u8>,
    /// Number of bits in the filter.
    num_bits: usize,
    /// Number of hash functions to use.
    num_hashes: u32,
    /// Number of elements inserted.
    count: usize,
}

impl BloomFilter {
    /// Create a new Bloom filter optimized for `expected_items`
    /// with the given `false_positive_rate`.
    ///
    /// # Formulas
    /// - Optimal bits: `m = -n * ln(p) / (ln(2)^2)`
    /// - Optimal hashes: `k = (m/n) * ln(2)`
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let expected_items = expected_items.max(1);
        let fp_rate = false_positive_rate.clamp(0.0001, 0.5);

        // Calculate optimal number of bits
        let num_bits =
            (-(expected_items as f64) * fp_rate.ln() / (2.0_f64.ln().powi(2))).ceil() as usize;
        let num_bits = num_bits.max(64); // minimum 64 bits

        // Calculate optimal number of hash functions
        let num_hashes = ((num_bits as f64 / expected_items as f64) * 2.0_f64.ln()).ceil() as u32;
        let num_hashes = num_hashes.clamp(2, 16);

        let num_bytes = num_bits.div_ceil(8);

        Self {
            bits: vec![0u8; num_bytes],
            num_bits,
            num_hashes,
            count: 0,
        }
    }

    /// Create a Bloom filter with explicit parameters.
    pub fn with_params(num_bits: usize, num_hashes: u32) -> Self {
        let num_bytes = num_bits.div_ceil(8);
        Self {
            bits: vec![0u8; num_bytes],
            num_bits,
            num_hashes: num_hashes.clamp(2, 16),
            count: 0,
        }
    }

    /// Insert a key into the Bloom filter.
    pub fn insert(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let bit_index = self.hash_index(key, i);
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            self.bits[byte_index] |= 1 << bit_offset;
        }
        self.count += 1;
    }

    /// Check if a key **may** be in the set.
    /// - Returns `false` → key is **definitely not** in the set
    /// - Returns `true` → key is **probably** in the set (may be false positive)
    pub fn may_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.num_hashes {
            let bit_index = self.hash_index(key, i);
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            if self.bits[byte_index] & (1 << bit_offset) == 0 {
                return false;
            }
        }
        true
    }

    /// Returns the number of elements inserted.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the number of bits in the filter.
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Returns the number of hash functions used.
    pub fn num_hashes(&self) -> u32 {
        self.num_hashes
    }

    /// Returns the approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.bits.len()
    }

    /// Estimated false positive rate based on current fill.
    pub fn estimated_fpr(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let k = self.num_hashes as f64;
        let m = self.num_bits as f64;
        let n = self.count as f64;
        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// Generate a bit index using double hashing.
    /// Uses the technique: `h(i) = h1 + i * h2` (mod m)
    /// where h1 and h2 are derived from two independent hashes.
    fn hash_index(&self, key: &[u8], i: u32) -> usize {
        let h1 = self.hash_with_seed(key, 0);
        let h2 = self.hash_with_seed(key, 0xDEADBEEF);
        let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
        (combined % self.num_bits as u64) as usize
    }

    /// Hash a key with a given seed using SipHash.
    fn hash_with_seed(&self, key: &[u8], seed: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_contains() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(b"hello");
        bf.insert(b"world");

        assert!(bf.may_contain(b"hello"));
        assert!(bf.may_contain(b"world"));
        assert_eq!(bf.count(), 2);
    }

    #[test]
    fn test_definitely_not_contains() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(b"alpha");
        bf.insert(b"bravo");

        // These should almost certainly return false
        // (with 100-item filter and only 2 items, FPR is negligible)
        let mut false_positives = 0;
        for i in 0..1000 {
            let key = format!("nonexistent_key_{}", i);
            if bf.may_contain(key.as_bytes()) {
                false_positives += 1;
            }
        }

        // With FPR=0.01, we expect ~10 false positives out of 1000
        // But with only 2 items in a 100-item filter, it should be much less
        assert!(
            false_positives < 50,
            "Too many false positives: {}",
            false_positives
        );
    }

    #[test]
    fn test_no_false_negatives() {
        let mut bf = BloomFilter::new(1000, 0.01);

        // Insert 500 keys
        for i in 0..500 {
            let key = format!("key_{}", i);
            bf.insert(key.as_bytes());
        }

        // ALL inserted keys must be found (zero false negatives)
        for i in 0..500 {
            let key = format!("key_{}", i);
            assert!(
                bf.may_contain(key.as_bytes()),
                "False negative for key: {}",
                key
            );
        }
    }

    #[test]
    fn test_estimated_fpr() {
        let mut bf = BloomFilter::new(100, 0.01);
        assert_eq!(bf.estimated_fpr(), 0.0); // empty filter

        for i in 0..100 {
            bf.insert(format!("k{}", i).as_bytes());
        }

        let fpr = bf.estimated_fpr();
        assert!(fpr > 0.0);
        assert!(fpr < 0.1); // should be around 0.01
    }

    #[test]
    fn test_memory_usage() {
        let bf = BloomFilter::new(1000, 0.01);
        assert!(bf.memory_usage() > 0);
        assert!(bf.num_bits() >= 64);
        assert!(bf.num_hashes() >= 2);
    }
}
