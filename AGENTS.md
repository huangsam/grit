# Grit - AI Development Guide

## Project Overview

Grit is a high-performance, from-scratch implementation of Git's core plumbing operations in Rust. It focuses on low-level version control primitives (objects, trees, commits) with aggressive optimizations like LRU caching, parallel processing, and buffered I/O. The goal is maximum performance while maintaining compatibility with standard Git repositories.

**Key Characteristics:**
- **Plumbing-Only**: Implements Git's low-level operations (no porcelain like `git add` or `git status` yet).
- **Performance-Focused**: 95%+ faster hash ops, 97%+ faster object reads via caching.
- **Compatible**: Uses Git's object formats, SHA-1 hashing, and directory structures.
- **Minimal Dependencies**: Core deps include `clap` (CLI), `flate2` (compression), `sha1`, `hex`, `rayon` (parallelism), `lru`, `lazy_static`.

## Architecture

Grit follows Git's object model with a modular design:

- **Objects**: Blobs (file content), Trees (directory structure), Commits (history snapshots).
- **Caching**: Multi-layer LRU system for hashes, decompressed objects, and parsed trees.
- **Parallelism**: Rayon for tree operations on multi-core systems.
- **I/O**: Buffered operations for efficiency.

### Core Modules

```
src/
├── lib.rs: Main library entry with docs and exports
├── main.rs: CLI implementation using Clap
├── repository.rs: Repo init and management (.grit/ dir)
├── error.rs: Comprehensive error types (GritError)
├── cache.rs: Thread-safe LRU caching
└── plumbing/
    ├── objects.rs: Store/read Git objects with compression
    ├── trees.rs: Create trees from dirs (parallel traversal)
    ├── commits.rs: Create commits, manage refs/history
    └── checkout.rs: Restore working dir from snapshots
```

## CLI Commands

Grit provides a Git-like CLI for plumbing operations:

- `grit init`: Initialize a new repository (creates `.grit/`).
- `grit hash-object <file>`: Store file as blob, print SHA-1.
- `grit cat-file <hash>`: Display object content (blob raw, tree/commit pretty-printed).
- `grit write-tree`: Create tree from current directory, print SHA-1.
- `grit commit -m <msg>`: Create commit from current tree, update HEAD.
- `grit checkout <hash>`: Restore working directory from tree/commit.

## Dependencies (Cargo.toml)

- **Core**: `clap` (CLI parsing), `flate2` (zlib compression), `sha1` (hashing), `hex` (hex encoding).
- **Performance**: `rayon` (parallelism), `lru` (caching), `lazy_static` (statics).
- **Dev**: `tempfile`, `proptest` (testing), `criterion` (benchmarking).

## Extending Grit

### Adding a New Plumbing Command
1. Add to `Commands` enum in `main.rs`.
2. Implement in `main()` match arm, calling a plumbing function.
3. Place logic in relevant `plumbing/` module.
4. Use `GritError` for errors; add tests.

### Future Porcelain Features
- Staging area, branches, merges, remotes.

**Guidelines**: Maintain performance (caching/parallelism), ensure Git compatibility, add docs/tests.
