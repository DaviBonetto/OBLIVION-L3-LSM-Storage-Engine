//! OBLIVION - Performance Benchmarks
//! Measures throughput of core engine operations using Criterion.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_memtable_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable");

    // Benchmark: Sequential inserts
    group.bench_function("insert_1000", |b| {
        b.iter(|| {
            let mut table = oblivion::engine::memtable::MemTable::new();
            for i in 0..1000 {
                let key = format!("key_{:06}", i).into_bytes();
                let value = format!("value_{:06}", i).into_bytes();
                table.insert(black_box(key), black_box(value));
            }
        });
    });

    // Benchmark: Point lookups
    group.bench_function("get_hit", |b| {
        let mut table = oblivion::engine::memtable::MemTable::new();
        for i in 0..1000 {
            let key = format!("key_{:06}", i).into_bytes();
            let value = format!("value_{:06}", i).into_bytes();
            table.insert(key, value);
        }
        b.iter(|| {
            let key = b"key_000500";
            black_box(table.get(key));
        });
    });

    // Benchmark: Point lookup miss
    group.bench_function("get_miss", |b| {
        let mut table = oblivion::engine::memtable::MemTable::new();
        for i in 0..1000 {
            let key = format!("key_{:06}", i).into_bytes();
            let value = format!("value_{:06}", i).into_bytes();
            table.insert(key, value);
        }
        b.iter(|| {
            let key = b"nonexistent_key";
            black_box(table.get(key));
        });
    });

    // Benchmark: Full scan
    group.bench_function("scan_1000", |b| {
        let mut table = oblivion::engine::memtable::MemTable::new();
        for i in 0..1000 {
            let key = format!("key_{:06}", i).into_bytes();
            let value = format!("value_{:06}", i).into_bytes();
            table.insert(key, value);
        }
        b.iter(|| {
            black_box(table.scan());
        });
    });

    // Benchmark: Delete with tombstone
    group.bench_function("delete_1000", |b| {
        b.iter(|| {
            let mut table = oblivion::engine::memtable::MemTable::new();
            for i in 0..1000 {
                let key = format!("key_{:06}", i).into_bytes();
                let value = format!("value_{:06}", i).into_bytes();
                table.insert(key, value);
            }
            for i in 0..1000 {
                let key = format!("key_{:06}", i).into_bytes();
                table.delete(key);
            }
        });
    });

    group.finish();
}

fn bench_bloom_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter");

    group.bench_function("insert_1000", |b| {
        b.iter(|| {
            let mut bf = oblivion::engine::bloom::BloomFilter::new(1000, 0.01);
            for i in 0..1000 {
                let key = format!("key_{:06}", i);
                bf.insert(black_box(key.as_bytes()));
            }
        });
    });

    group.bench_function("lookup_hit", |b| {
        let mut bf = oblivion::engine::bloom::BloomFilter::new(1000, 0.01);
        for i in 0..1000 {
            let key = format!("key_{:06}", i);
            bf.insert(key.as_bytes());
        }
        b.iter(|| {
            black_box(bf.may_contain(b"key_000500"));
        });
    });

    group.bench_function("lookup_miss", |b| {
        let mut bf = oblivion::engine::bloom::BloomFilter::new(1000, 0.01);
        for i in 0..1000 {
            let key = format!("key_{:06}", i);
            bf.insert(key.as_bytes());
        }
        b.iter(|| {
            black_box(bf.may_contain(b"definitely_not_here"));
        });
    });

    group.finish();
}

fn bench_wal_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal");

    group.bench_function("append_100", |b| {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("bench.wal");
        let mut wal = oblivion::engine::wal::WriteAheadLog::open(wal_path).unwrap();

        b.iter(|| {
            for i in 0..100 {
                let key = format!("key_{:06}", i).into_bytes();
                let value = format!("value_{:06}", i).into_bytes();
                wal.append_put(black_box(&key), black_box(&value)).unwrap();
            }
        });
    });

    group.finish();
}

fn bench_engine_e2e(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_e2e");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("put_get_cycle", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let dir = tempfile::tempdir().unwrap();
                    let config = oblivion::config::Config {
                        data_dir: dir.path().to_path_buf(),
                        memtable_max_size: 64 * 1024, // 64KB
                        sync_writes: true,
                    };
                    let mut engine = oblivion::engine::Oblivion::open(config).unwrap();

                    for i in 0..size {
                        let key = format!("key_{:06}", i).into_bytes();
                        let value = format!("value_{:06}", i).into_bytes();
                        engine.put(key, value).unwrap();
                    }

                    for i in 0..size {
                        let key = format!("key_{:06}", i);
                        black_box(engine.get(key.as_bytes()));
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_memtable_operations,
    bench_bloom_filter,
    bench_wal_operations,
    bench_engine_e2e
);
criterion_main!(benches);
