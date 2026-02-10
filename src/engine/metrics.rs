//! OBLIVION - Engine Metrics & Observability
//! Provides atomic counters for tracking engine operations
//! in a lock-free, thread-safe manner using `AtomicU64`.
//!
//! These metrics enable runtime introspection into engine
//! behavior without impacting performance.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Atomic operation counters for the Oblivion engine.
///
/// All counters use `Ordering::Relaxed` since we only need
/// eventual consistency for observability — not synchronization.
#[derive(Debug)]
pub struct EngineMetrics {
    /// Total number of `put` operations.
    pub puts: AtomicU64,
    /// Total number of `get` operations.
    pub gets: AtomicU64,
    /// Total number of `delete` operations.
    pub deletes: AtomicU64,
    /// Total number of `scan` operations.
    pub scans: AtomicU64,
    /// Total number of flush (MemTable → SSTable) events.
    pub flushes: AtomicU64,
    /// Total bytes written (keys + values).
    pub bytes_written: AtomicU64,
    /// Total bytes read (values returned by get).
    pub bytes_read: AtomicU64,
    /// Number of WAL recovery operations.
    pub wal_recoveries: AtomicU64,
    /// Timestamp when the engine was opened.
    engine_started: Instant,
}

impl EngineMetrics {
    /// Create a new metrics instance with all counters at zero.
    pub fn new() -> Self {
        Self {
            puts: AtomicU64::new(0),
            gets: AtomicU64::new(0),
            deletes: AtomicU64::new(0),
            scans: AtomicU64::new(0),
            flushes: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            wal_recoveries: AtomicU64::new(0),
            engine_started: Instant::now(),
        }
    }

    /// Record a put operation.
    pub fn record_put(&self, key_size: usize, value_size: usize) {
        self.puts.fetch_add(1, Ordering::Relaxed);
        self.bytes_written
            .fetch_add((key_size + value_size) as u64, Ordering::Relaxed);
    }

    /// Record a get operation.
    pub fn record_get(&self, value_size: Option<usize>) {
        self.gets.fetch_add(1, Ordering::Relaxed);
        if let Some(size) = value_size {
            self.bytes_read.fetch_add(size as u64, Ordering::Relaxed);
        }
    }

    /// Record a delete operation.
    pub fn record_delete(&self) {
        self.deletes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a scan operation.
    pub fn record_scan(&self) {
        self.scans.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a flush event.
    pub fn record_flush(&self) {
        self.flushes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a WAL recovery.
    pub fn record_recovery(&self) {
        self.wal_recoveries.fetch_add(1, Ordering::Relaxed);
    }

    /// Get engine uptime in seconds.
    pub fn uptime_secs(&self) -> f64 {
        self.engine_started.elapsed().as_secs_f64()
    }

    /// Get total number of operations (puts + gets + deletes + scans).
    pub fn total_ops(&self) -> u64 {
        self.puts.load(Ordering::Relaxed)
            + self.gets.load(Ordering::Relaxed)
            + self.deletes.load(Ordering::Relaxed)
            + self.scans.load(Ordering::Relaxed)
    }

    /// Get operations per second since engine start.
    pub fn ops_per_sec(&self) -> f64 {
        let uptime = self.uptime_secs();
        if uptime < 0.001 {
            return 0.0;
        }
        self.total_ops() as f64 / uptime
    }

    /// Format metrics as a human-readable report.
    pub fn report(&self) -> String {
        format!(
            "\n═══ OBLIVION Engine Metrics ═══\n\
             Operations:\n\
               puts:      {}\n\
               gets:      {}\n\
               deletes:   {}\n\
               scans:     {}\n\
               flushes:   {}\n\
             Throughput:\n\
               total ops: {}\n\
               ops/sec:   {:.2}\n\
             I/O:\n\
               written:   {} bytes\n\
               read:      {} bytes\n\
             Recovery:\n\
               wal recoveries: {}\n\
             Uptime: {:.2}s",
            self.puts.load(Ordering::Relaxed),
            self.gets.load(Ordering::Relaxed),
            self.deletes.load(Ordering::Relaxed),
            self.scans.load(Ordering::Relaxed),
            self.flushes.load(Ordering::Relaxed),
            self.total_ops(),
            self.ops_per_sec(),
            self.bytes_written.load(Ordering::Relaxed),
            self.bytes_read.load(Ordering::Relaxed),
            self.wal_recoveries.load(Ordering::Relaxed),
            self.uptime_secs(),
        )
    }
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_operations() {
        let m = EngineMetrics::new();

        m.record_put(5, 10);
        m.record_put(3, 7);
        m.record_get(Some(10));
        m.record_get(None); // cache miss
        m.record_delete();
        m.record_scan();
        m.record_flush();

        assert_eq!(m.puts.load(Ordering::Relaxed), 2);
        assert_eq!(m.gets.load(Ordering::Relaxed), 2);
        assert_eq!(m.deletes.load(Ordering::Relaxed), 1);
        assert_eq!(m.scans.load(Ordering::Relaxed), 1);
        assert_eq!(m.flushes.load(Ordering::Relaxed), 1);
        assert_eq!(m.bytes_written.load(Ordering::Relaxed), 25);
        assert_eq!(m.bytes_read.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_total_ops() {
        let m = EngineMetrics::new();
        m.record_put(1, 1);
        m.record_get(None);
        m.record_delete();
        m.record_scan();
        assert_eq!(m.total_ops(), 4);
    }

    #[test]
    fn test_report_format() {
        let m = EngineMetrics::new();
        m.record_put(10, 20);
        let report = m.report();
        assert!(report.contains("puts:"));
        assert!(report.contains("ops/sec:"));
        assert!(report.contains("written:"));
    }

    #[test]
    fn test_default() {
        let m = EngineMetrics::default();
        assert_eq!(m.total_ops(), 0);
    }
}
