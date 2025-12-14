use criterion::{Criterion, black_box, criterion_group, criterion_main};
use grit::plumbing::objects::{ObjectType, read_object, store_object};
use grit::repository::initialize_repo;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    initialize_repo(temp_dir.path()).unwrap();
    temp_dir
}

fn bench_store_object_small(c: &mut Criterion) {
    // Benchmark storing small objects (file content compression and hashing)
    let test_dir = setup_test_repo();
    let content = b"Hello, World! This is a small test file.";

    c.bench_function("store_object_small", |b| {
        b.iter(|| {
            let hash = store_object(black_box(content), ObjectType::Blob, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_store_object_medium(c: &mut Criterion) {
    // Benchmark storing medium-sized objects (100KB content)
    let test_dir = setup_test_repo();
    let content = vec![b'A'; 100 * 1024]; // 100KB

    c.bench_function("store_object_medium", |b| {
        b.iter(|| {
            let hash =
                store_object(black_box(&content), ObjectType::Blob, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_store_object_large(c: &mut Criterion) {
    // Benchmark storing large objects (1MB content)
    let test_dir = setup_test_repo();
    let content = vec![b'B'; 1024 * 1024]; // 1MB

    c.bench_function("store_object_large", |b| {
        b.iter(|| {
            let hash =
                store_object(black_box(&content), ObjectType::Blob, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_read_object_small(c: &mut Criterion) {
    // Benchmark reading small objects (decompression and parsing)
    let test_dir = setup_test_repo();
    let content = b"Hello, World! This is a small test file.";
    let hash = store_object(content, ObjectType::Blob, test_dir.path()).unwrap();

    c.bench_function("read_object_small", |b| {
        b.iter(|| {
            let obj = read_object(black_box(&hash), test_dir.path()).unwrap();
            black_box(obj);
        })
    });
}

fn bench_read_object_medium(c: &mut Criterion) {
    // Benchmark reading medium-sized objects (100KB content)
    let test_dir = setup_test_repo();
    let content = vec![b'A'; 100 * 1024]; // 100KB
    let hash = store_object(&content, ObjectType::Blob, test_dir.path()).unwrap();

    c.bench_function("read_object_medium", |b| {
        b.iter(|| {
            let obj = read_object(black_box(&hash), test_dir.path()).unwrap();
            black_box(obj);
        })
    });
}

fn bench_read_object_large(c: &mut Criterion) {
    // Benchmark reading large objects (1MB content)
    let test_dir = setup_test_repo();
    let content = vec![b'B'; 1024 * 1024]; // 1MB
    let hash = store_object(&content, ObjectType::Blob, test_dir.path()).unwrap();

    c.bench_function("read_object_large", |b| {
        b.iter(|| {
            let obj = read_object(black_box(&hash), test_dir.path()).unwrap();
            black_box(obj);
        })
    });
}

fn bench_store_read_roundtrip(c: &mut Criterion) {
    // Benchmark complete store-then-read cycle for 10KB objects
    let test_dir = setup_test_repo();

    c.bench_function("store_read_roundtrip_10kb", |b| {
        b.iter(|| {
            let content = vec![b'C'; 10 * 1024];
            let hash =
                store_object(black_box(&content), ObjectType::Blob, test_dir.path()).unwrap();
            let obj = read_object(black_box(&hash), test_dir.path()).unwrap();
            black_box(obj);
        })
    });
}

criterion_group!(
    benches,
    bench_store_object_small,
    bench_store_object_medium,
    bench_store_object_large,
    bench_read_object_small,
    bench_read_object_medium,
    bench_read_object_large,
    bench_store_read_roundtrip
);
criterion_main!(benches);
