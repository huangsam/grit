use criterion::{Criterion, black_box, criterion_group, criterion_main};
use grit::plumbing::trees::write_tree_from_index;
use grit::plumbing::index::{Index, create_index_entry};
use grit::plumbing::objects::{store_object, ObjectType};
use grit::repository::initialize_repo;
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

fn populate_index_recursive(path: &Path, repo_root: &Path, index: &mut Index) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == ".grit" || file_name == "target" || file_name == ".git" {
            continue;
        }

        let file_path = entry.path();
        if file_path.is_dir() {
            populate_index_recursive(&file_path, repo_root, index);
        } else if file_path.is_file() {
            let content = fs::read(&file_path).unwrap();
            let hash = store_object(&content, ObjectType::Blob, repo_root).unwrap();
            let hash_bytes = hex::decode(hash).unwrap();
            let mut hash_array = [0u8; 20];
            hash_array.copy_from_slice(&hash_bytes);

            let index_entry = create_index_entry(&file_path, &hash_array, repo_root).unwrap();
            index.add_entry(index_entry);
        }
    }
}

fn create_index_for_bench(path: &Path, repo_root: &Path) -> Index {
    let mut index = Index::new();
    populate_index_recursive(path, repo_root, &mut index);
    index
}

fn bench_write_tree_small(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 10, 1);
    let index = create_index_for_bench(test_dir.path(), test_dir.path());

    c.bench_function("write_tree_small", |b| {
        b.iter(|| {
            let hash = write_tree_from_index(&index, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_write_tree_medium(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 100, 1);
    let index = create_index_for_bench(test_dir.path(), test_dir.path());

    c.bench_function("write_tree_medium", |b| {
        b.iter(|| {
            let hash = write_tree_from_index(&index, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_write_tree_large(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_test_files(test_dir.path(), 1000, 1);
    let index = create_index_for_bench(test_dir.path(), test_dir.path());

    c.bench_function("write_tree_large", |b| {
        b.iter(|| {
            let hash = write_tree_from_index(&index, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_write_tree_nested(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_nested_structure(test_dir.path(), 3, 5);
    let index = create_index_for_bench(test_dir.path(), test_dir.path());

    c.bench_function("write_tree_nested", |b| {
        b.iter(|| {
            let hash = write_tree_from_index(&index, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

fn bench_write_tree_deep(c: &mut Criterion) {
    let test_dir = setup_test_repo();
    create_nested_structure(test_dir.path(), 5, 2);
    let index = create_index_for_bench(test_dir.path(), test_dir.path());

    c.bench_function("write_tree_deep", |b| {
        b.iter(|| {
            let hash = write_tree_from_index(&index, test_dir.path()).unwrap();
            black_box(hash);
        })
    });
}

criterion_group!(
    benches,
    bench_write_tree_small,
    bench_write_tree_medium,
    bench_write_tree_large,
    bench_write_tree_nested,
    bench_write_tree_deep
);
criterion_main!(benches);
