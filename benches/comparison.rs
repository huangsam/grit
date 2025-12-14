use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn grit_binary_path() -> PathBuf {
    let mut path = std::env::current_dir().unwrap();
    path.push("target/release/grit");
    path
}

fn setup_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    // Create sample files for the repo
    std::fs::write(dir.path().join("file1.txt"), "Hello, world! This is file 1.").unwrap();
    std::fs::write(dir.path().join("file2.txt"), "Hello, world! This is file 2.").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    std::fs::write(dir.path().join("subdir/file3.txt"), "Hello, world! This is file 3.").unwrap();
    dir
}

fn run_grit_command(dir: &std::path::Path, args: &[&str]) {
    Command::new(grit_binary_path())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to run Grit command");
}

fn run_git_command(dir: &std::path::Path, args: &[&str]) {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to run Git command");
}

fn bench_grit_init(c: &mut Criterion) {
    c.bench_function("grit_init", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
        })
    });
}

fn bench_git_init(c: &mut Criterion) {
    c.bench_function("git_init", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
        })
    });
}

fn bench_grit_add(c: &mut Criterion) {
    c.bench_function("grit_add", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
        })
    });
}

fn bench_git_add(c: &mut Criterion) {
    c.bench_function("git_add", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
        })
    });
}

fn bench_grit_status(c: &mut Criterion) {
    c.bench_function("grit_status", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["status"]);
        })
    });
}

fn bench_git_status(c: &mut Criterion) {
    c.bench_function("git_status", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["status"]);
        })
    });
}

fn bench_grit_commit(c: &mut Criterion) {
    c.bench_function("grit_commit", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["commit", "-m", "Initial commit"]);
        })
    });
}

fn bench_git_commit(c: &mut Criterion) {
    c.bench_function("git_commit", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Initial commit"]);
        })
    });
}

criterion_group!(
    benches,
    bench_grit_init,
    bench_git_init,
    bench_grit_add,
    bench_git_add,
    bench_grit_status,
    bench_git_status,
    bench_grit_commit,
    bench_git_commit
);
criterion_main!(benches);
