use criterion::{Criterion, black_box, criterion_group, criterion_main};
use grit::plumbing::diff::{compare_trees, get_file_deltas};
use grit::plumbing::index::{Index, create_index_entry};
use grit::plumbing::objects::{ObjectType, store_object};
use grit::plumbing::trees::write_tree_from_index;
use grit::repository::{Repository, initialize_repo};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    initialize_repo(temp_dir.path()).unwrap();
    temp_dir
}

fn populate_index_from_dir(dir: &Path, repo_root: &Path, index: &mut Index) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".grit" || name == "target" || name == ".git" {
            continue;
        }
        let file_path = entry.path();
        if file_path.is_dir() {
            populate_index_from_dir(&file_path, repo_root, index);
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

fn generate_lines(count: usize, prefix: &str) -> String {
    let mut s = String::with_capacity(count * 40);
    for i in 0..count {
        s.push_str(&format!(
            "{prefix} line {i}: sample text for benchmarking diffs\n"
        ));
    }
    s
}

fn bench_get_file_deltas_small(c: &mut Criterion) {
    // Benchmark diffing small files (~20 lines) with a few edits
    let content_a = generate_lines(20, "base");
    let mut content_b = generate_lines(20, "base");
    content_b = content_b.replace("base line 5:", "modified line 5:");
    content_b = content_b.replace("base line 12:", "modified line 12:");
    let path = Path::new("file.txt");

    c.bench_function("diff_deltas_small", |b| {
        b.iter(|| {
            let res = get_file_deltas(
                black_box(&content_a),
                black_box(&content_b),
                black_box(path),
            );
            black_box(res);
        })
    });
}

fn bench_get_file_deltas_medium(c: &mut Criterion) {
    // Benchmark diffing medium files (~1000 lines) with sparse edits
    let content_a = generate_lines(1000, "base");
    let mut content_b = generate_lines(1000, "base");
    for i in (0..1000).step_by(20) {
        content_b = content_b.replace(&format!("base line {i}:"), &format!("modified line {i}:"));
    }
    let path = Path::new("file.txt");

    c.bench_function("diff_deltas_medium", |b| {
        b.iter(|| {
            let res = get_file_deltas(
                black_box(&content_a),
                black_box(&content_b),
                black_box(path),
            );
            black_box(res);
        })
    });
}

fn bench_get_file_deltas_large(c: &mut Criterion) {
    // Benchmark diffing large files (~5000 lines) with scattered modifications
    let content_a = generate_lines(5000, "base");
    let mut content_b = generate_lines(5000, "base");
    for i in (0..5000).step_by(50) {
        content_b = content_b.replace(&format!("base line {i}:"), &format!("modified line {i}:"));
    }
    let path = Path::new("file.txt");

    c.bench_function("diff_deltas_large", |b| {
        b.iter(|| {
            let res = get_file_deltas(
                black_box(&content_a),
                black_box(&content_b),
                black_box(path),
            );
            black_box(res);
        })
    });
}

fn bench_get_file_deltas_disjoint(c: &mut Criterion) {
    // Benchmark diffing completely disjoint files (all lines differing)
    let content_a = generate_lines(500, "file_a");
    let content_b = generate_lines(500, "file_b");
    let path = Path::new("file.txt");

    c.bench_function("diff_deltas_disjoint", |b| {
        b.iter(|| {
            let res = get_file_deltas(
                black_box(&content_a),
                black_box(&content_b),
                black_box(path),
            );
            black_box(res);
        })
    });
}

fn bench_compare_trees_flat(c: &mut Criterion) {
    // Benchmark tree comparison on flat trees with additions, modifications, and deletions
    let test_dir = setup_test_repo();
    let repo = Repository::new(test_dir.path());

    for i in 0..50 {
        fs::write(
            test_dir.path().join(format!("file_{i}.txt")),
            format!("content {i}"),
        )
        .unwrap();
    }
    let mut index_a = Index::new();
    populate_index_from_dir(test_dir.path(), test_dir.path(), &mut index_a);
    let tree_a = write_tree_from_index(&index_a, test_dir.path()).unwrap();

    for i in 0..5 {
        fs::write(
            test_dir.path().join(format!("file_{i}.txt")),
            format!("modified content {i}"),
        )
        .unwrap();
    }
    for i in 45..50 {
        fs::remove_file(test_dir.path().join(format!("file_{i}.txt"))).unwrap();
    }
    for i in 50..55 {
        fs::write(
            test_dir.path().join(format!("file_{i}.txt")),
            format!("new content {i}"),
        )
        .unwrap();
    }
    let mut index_b = Index::new();
    populate_index_from_dir(test_dir.path(), test_dir.path(), &mut index_b);
    let tree_b = write_tree_from_index(&index_b, test_dir.path()).unwrap();

    c.bench_function("compare_trees_flat", |b| {
        b.iter(|| {
            let diffs = compare_trees(
                black_box(&repo),
                black_box(&tree_a),
                black_box(&tree_b),
                black_box(Path::new("")),
            )
            .unwrap();
            black_box(diffs);
        })
    });
}

fn bench_compare_trees_nested(c: &mut Criterion) {
    // Benchmark tree comparison across multi-level nested directories
    let test_dir = setup_test_repo();
    let repo = Repository::new(test_dir.path());

    for d in 0..3 {
        let subdir = test_dir.path().join(format!("sub_{d}"));
        fs::create_dir(&subdir).unwrap();
        for f in 0..10 {
            fs::write(
                subdir.join(format!("file_{f}.txt")),
                format!("content {d}_{f}"),
            )
            .unwrap();
        }
    }
    let mut index_a = Index::new();
    populate_index_from_dir(test_dir.path(), test_dir.path(), &mut index_a);
    let tree_a = write_tree_from_index(&index_a, test_dir.path()).unwrap();

    for d in 0..3 {
        let subdir = test_dir.path().join(format!("sub_{d}"));
        fs::write(subdir.join("file_0.txt"), format!("modified {d}")).unwrap();
    }
    fs::write(
        test_dir.path().join("sub_0").join("new_file.txt"),
        "brand new file",
    )
    .unwrap();

    let mut index_b = Index::new();
    populate_index_from_dir(test_dir.path(), test_dir.path(), &mut index_b);
    let tree_b = write_tree_from_index(&index_b, test_dir.path()).unwrap();

    c.bench_function("compare_trees_nested", |b| {
        b.iter(|| {
            let diffs = compare_trees(
                black_box(&repo),
                black_box(&tree_a),
                black_box(&tree_b),
                black_box(Path::new("")),
            )
            .unwrap();
            black_box(diffs);
        })
    });
}

fn bench_compare_trees_identical(c: &mut Criterion) {
    // Benchmark fast-path comparison of identical trees
    let test_dir = setup_test_repo();
    let repo = Repository::new(test_dir.path());

    for i in 0..20 {
        fs::write(
            test_dir.path().join(format!("file_{i}.txt")),
            format!("content {i}"),
        )
        .unwrap();
    }
    let mut index = Index::new();
    populate_index_from_dir(test_dir.path(), test_dir.path(), &mut index);
    let tree_hash = write_tree_from_index(&index, test_dir.path()).unwrap();

    c.bench_function("compare_trees_identical", |b| {
        b.iter(|| {
            let diffs = compare_trees(
                black_box(&repo),
                black_box(&tree_hash),
                black_box(&tree_hash),
                black_box(Path::new("")),
            )
            .unwrap();
            black_box(diffs);
        })
    });
}

criterion_group!(
    benches,
    bench_get_file_deltas_small,
    bench_get_file_deltas_medium,
    bench_get_file_deltas_large,
    bench_get_file_deltas_disjoint,
    bench_compare_trees_flat,
    bench_compare_trees_nested,
    bench_compare_trees_identical
);
criterion_main!(benches);
