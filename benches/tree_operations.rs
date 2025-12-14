use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crust::plumbing::trees::make_snapshot;
use crust::repository::initialize_repo;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    initialize_repo(temp_dir.path()).unwrap();
    temp_dir
}

fn create_test_files(dir: &Path, count: usize, size_kb: usize) {
    for i in 0..count {
        let filename = format!("file_{}.txt", i);
        let content = vec![b'X'; size_kb * 1024];
        fs::write(dir.join(filename), content).unwrap();
    }
}

fn create_nested_structure(dir: &Path, depth: usize, files_per_dir: usize) {
    if depth == 0 {
        create_test_files(dir, files_per_dir, 1);
        return;
    }

    // Create files in current directory
    create_test_files(dir, files_per_dir, 1);

    // Create subdirectories
    for i in 0..3 {
        let subdir = dir.join(format!("dir_{}", i));
        fs::create_dir(&subdir).unwrap();
        create_nested_structure(&subdir, depth - 1, files_per_dir);
    }
}

fn bench_make_snapshot_small(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 10, 1); // 10 files, 1KB each

    c.bench_function("make_snapshot_small", |b| {
        b.iter(|| {
            let hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_make_snapshot_medium(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 100, 1); // 100 files, 1KB each

    c.bench_function("make_snapshot_medium", |b| {
        b.iter(|| {
            let hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_make_snapshot_large(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 1000, 1); // 1000 files, 1KB each

    c.bench_function("make_snapshot_large", |b| {
        b.iter(|| {
            let hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_make_snapshot_nested(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_nested_structure(test_dir.path(), 3, 5); // 3 levels deep, 5 files per dir

    c.bench_function("make_snapshot_nested", |b| {
        b.iter(|| {
            let hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_make_snapshot_deep(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_nested_structure(test_dir.path(), 5, 2); // 5 levels deep, 2 files per dir

    c.bench_function("make_snapshot_deep", |b| {
        b.iter(|| {
            let hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

criterion_group!(
    benches,
    bench_make_snapshot_small,
    bench_make_snapshot_medium,
    bench_make_snapshot_large,
    bench_make_snapshot_nested,
    bench_make_snapshot_deep
);
criterion_main!(benches);
