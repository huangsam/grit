use assert_cmd::cargo::cargo_bin;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn grit_binary_path() -> PathBuf {
    cargo_bin("grit")
}

fn setup_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    // Create sample files for the repo
    std::fs::write(
        dir.path().join("file1.txt"),
        "Hello, world! This is file 1.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("file2.txt"),
        "Hello, world! This is file 2.",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    std::fs::write(
        dir.path().join("subdir/file3.txt"),
        "Hello, world! This is file 3.",
    )
    .unwrap();
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
        .envs([
            ("GIT_AUTHOR_NAME", "Benchmark"),
            ("GIT_AUTHOR_EMAIL", "bench@grit.local"),
            ("GIT_COMMITTER_NAME", "Benchmark"),
            ("GIT_COMMITTER_EMAIL", "bench@grit.local"),
            ("GIT_PAGER", "cat"),
        ])
        .current_dir(dir)
        .output()
        .expect("Failed to run Git command");
}

fn bench_grit_init(c: &mut Criterion) {
    // Benchmark Grit repository initialization performance
    c.bench_function("grit_init", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
        })
    });
}

fn bench_git_init(c: &mut Criterion) {
    // Benchmark Git repository initialization performance for comparison
    c.bench_function("git_init", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
        })
    });
}

fn bench_grit_add(c: &mut Criterion) {
    // Benchmark Grit file staging performance
    c.bench_function("grit_add", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
        })
    });
}

fn bench_git_add(c: &mut Criterion) {
    // Benchmark Git file staging performance for comparison
    c.bench_function("git_add", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
        })
    });
}

fn bench_grit_status(c: &mut Criterion) {
    // Benchmark Grit status command performance
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
    // Benchmark Git status command performance for comparison
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
    // Benchmark Grit commit creation performance
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
    // Benchmark Git commit creation performance for comparison
    c.bench_function("git_commit", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Initial commit"]);
        })
    });
}

fn bench_grit_diff(c: &mut Criterion) {
    // Benchmark Grit diff performance on unstaged modifications
    c.bench_function("grit_diff", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["commit", "-m", "Initial commit"]);
            std::fs::write(
                dir.path().join("file1.txt"),
                "Hello, world! This is file 1 with diff modifications.",
            )
            .unwrap();
            run_grit_command(dir.path(), &["diff"]);
        })
    });
}

fn bench_git_diff(c: &mut Criterion) {
    // Benchmark Git diff performance on unstaged modifications for comparison
    c.bench_function("git_diff", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Initial commit"]);
            std::fs::write(
                dir.path().join("file1.txt"),
                "Hello, world! This is file 1 with diff modifications.",
            )
            .unwrap();
            run_git_command(dir.path(), &["diff"]);
        })
    });
}

fn bench_grit_log(c: &mut Criterion) {
    // Benchmark Grit commit log performance
    c.bench_function("grit_log", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["commit", "-m", "Initial commit"]);
            std::fs::write(dir.path().join("file1.txt"), "Second revision of file 1").unwrap();
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["commit", "-m", "Second commit"]);
            run_grit_command(dir.path(), &["log"]);
        })
    });
}

fn bench_git_log(c: &mut Criterion) {
    // Benchmark Git commit log performance for comparison
    c.bench_function("git_log", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Initial commit"]);
            std::fs::write(dir.path().join("file1.txt"), "Second revision of file 1").unwrap();
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Second commit"]);
            run_git_command(dir.path(), &["log"]);
        })
    });
}

fn bench_grit_checkout(c: &mut Criterion) {
    // Benchmark Grit branch checkout performance
    c.bench_function("grit_checkout", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_grit_command(dir.path(), &["init"]);
            run_grit_command(dir.path(), &["add", "."]);
            run_grit_command(dir.path(), &["commit", "-m", "Initial commit"]);
            run_grit_command(dir.path(), &["checkout", "-b", "feature"]);
        })
    });
}

fn bench_git_checkout(c: &mut Criterion) {
    // Benchmark Git branch checkout performance for comparison
    c.bench_function("git_checkout", |b| {
        b.iter(|| {
            let dir = black_box(setup_repo());
            run_git_command(dir.path(), &["init"]);
            run_git_command(dir.path(), &["add", "."]);
            run_git_command(dir.path(), &["commit", "-m", "Initial commit"]);
            run_git_command(dir.path(), &["checkout", "-b", "feature"]);
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
    bench_git_commit,
    bench_grit_diff,
    bench_git_diff,
    bench_grit_log,
    bench_git_log,
    bench_grit_checkout,
    bench_git_checkout
);
criterion_main!(benches);
