use criterion::{Criterion, black_box, criterion_group, criterion_main};
use grit::plumbing::index::{Index, IndexEntry, create_index_entry, read_index, write_index};
use grit::repository::initialize_repo;
use std::fs;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    initialize_repo(temp_dir.path()).unwrap();
    temp_dir
}

fn make_sample_entry(i: usize) -> IndexEntry {
    let mut hash = [0u8; 20];
    hash[0] = (i & 0xFF) as u8;
    hash[1] = ((i >> 8) & 0xFF) as u8;
    IndexEntry {
        ctime_sec: 1_700_000_000,
        ctime_nsec: 0,
        mtime_sec: 1_700_000_000,
        mtime_nsec: 0,
        dev: 1,
        ino: (1000 + i) as u32,
        mode: 0o100644,
        uid: 1000,
        gid: 1000,
        size: 1234,
        hash,
        flags: 0,
        path: format!("src/module_{:04}/file_{:04}.rs", i / 10, i),
    }
}

fn make_index(count: usize) -> Index {
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        entries.push(make_sample_entry(i));
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Index {
        entries,
        version: 2,
    }
}

fn bench_write_index_small(c: &mut Criterion) {
    // Benchmark writing small index (10 entries) to binary DIRC format
    let test_dir = setup_test_repo();
    let index = make_index(10);

    c.bench_function("write_index_10", |b| {
        b.iter(|| {
            write_index(black_box(&index), black_box(test_dir.path())).unwrap();
        })
    });
}

fn bench_write_index_medium(c: &mut Criterion) {
    // Benchmark writing medium index (200 entries) to binary DIRC format
    let test_dir = setup_test_repo();
    let index = make_index(200);

    c.bench_function("write_index_200", |b| {
        b.iter(|| {
            write_index(black_box(&index), black_box(test_dir.path())).unwrap();
        })
    });
}

fn bench_write_index_large(c: &mut Criterion) {
    // Benchmark writing large index (1000 entries) to binary DIRC format
    let test_dir = setup_test_repo();
    let index = make_index(1000);

    c.bench_function("write_index_1000", |b| {
        b.iter(|| {
            write_index(black_box(&index), black_box(test_dir.path())).unwrap();
        })
    });
}

fn bench_read_index_small(c: &mut Criterion) {
    // Benchmark reading small binary index (10 entries) with checksum validation
    let test_dir = setup_test_repo();
    let index = make_index(10);
    write_index(&index, test_dir.path()).unwrap();

    c.bench_function("read_index_10", |b| {
        b.iter(|| {
            let idx = read_index(black_box(test_dir.path())).unwrap();
            black_box(idx);
        })
    });
}

fn bench_read_index_medium(c: &mut Criterion) {
    // Benchmark reading medium binary index (200 entries) with checksum validation
    let test_dir = setup_test_repo();
    let index = make_index(200);
    write_index(&index, test_dir.path()).unwrap();

    c.bench_function("read_index_200", |b| {
        b.iter(|| {
            let idx = read_index(black_box(test_dir.path())).unwrap();
            black_box(idx);
        })
    });
}

fn bench_read_index_large(c: &mut Criterion) {
    // Benchmark reading large binary index (1000 entries) with checksum validation
    let test_dir = setup_test_repo();
    let index = make_index(1000);
    write_index(&index, test_dir.path()).unwrap();

    c.bench_function("read_index_1000", |b| {
        b.iter(|| {
            let idx = read_index(black_box(test_dir.path())).unwrap();
            black_box(idx);
        })
    });
}

fn bench_create_index_entry(c: &mut Criterion) {
    // Benchmark index entry construction from filesystem metadata
    let test_dir = setup_test_repo();
    let file_path = test_dir.path().join("bench_sample.txt");
    fs::write(&file_path, "sample content for stat benchmarking").unwrap();
    let hash = [0xabu8; 20];

    c.bench_function("create_index_entry", |b| {
        b.iter(|| {
            let entry = create_index_entry(
                black_box(&file_path),
                black_box(&hash),
                black_box(test_dir.path()),
            )
            .unwrap();
            black_box(entry);
        })
    });
}

fn bench_index_add_entry(c: &mut Criterion) {
    // Benchmark sorted entry insertion into existing 500-entry index
    let base_index = make_index(500);
    let new_entry = make_sample_entry(255);

    c.bench_function("index_add_entry_500", |b| {
        b.iter(|| {
            let mut idx = base_index.clone();
            idx.add_entry(black_box(new_entry.clone()));
            black_box(idx);
        })
    });
}

criterion_group!(
    benches,
    bench_write_index_small,
    bench_write_index_medium,
    bench_write_index_large,
    bench_read_index_small,
    bench_read_index_medium,
    bench_read_index_large,
    bench_create_index_entry,
    bench_index_add_entry
);
criterion_main!(benches);
