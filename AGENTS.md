# Grit - AI Development Guide

## Project Overview

Grit is a high-performance, from-scratch implementation of Git's core operations in Rust. While originally focused on low-level plumbing primitives (objects, trees, commits), it has evolved to include essential porcelain commands (`add`, `status`, `reset`). It features aggressive optimizations like LRU caching, parallel processing, and buffered I/O.

**Key Characteristics:**
- **Hybrid Architecture**: Implements both plumbing (low-level) and porcelain (high-level) commands.
- **Performance-Focused**: 95%+ faster hash ops, 97%+ faster object reads via caching.
- **Compatible**: Uses Git's object formats, SHA-1 hashing, directory structures, and **Index format**.
- **Minimal Dependencies**: Core deps include `clap`, `flate2`, `sha1`, `hex`, `glob`.

## Architecture

Grit follows Git's object model with a modular design:

- **Objects**: Blobs (file content), Trees (directory structure), Commits (history snapshots).
- **Index**: Fully compatible Git Index (staging area) implementation.
- **Caching**: Multi-layer LRU system for hashes, decompressed objects, and parsed trees.
- **Parallelism**: Rayon for tree operations on multi-core systems.

### Core Modules

```
src/
├── lib.rs: Main library entry with docs and exports
├── main.rs: CLI implementation using Clap
├── repository.rs: Repo init and management (.grit/ dir)
├── error.rs: Comprehensive error types (GritError)
├── cache.rs: Thread-safe LRU caching
├── commands/ (Porcelain)
│   ├── add.rs: Staging files (updates Index, respects .gritignore)
│   ├── status.rs: Working tree vs Index vs HEAD comparison (hides ignored files)
│   ├── reset.rs: Moving HEAD and updating Index/Working Tree
│   └── diff.rs: Show changes between commits
└── plumbing/ (Core)
    ├── objects.rs: Store/read Git objects with compression
    ├── trees.rs: Create trees from Index (write-tree)
    ├── commits.rs: Create commits, manage refs/history
    ├── checkout.rs: Restore working dir from snapshots
    ├── index.rs: Git Index binary format reader/writer
    ├── ignores.rs: Load and match .gritignore patterns
    └── diff.rs: Core diff algorithm (Myers)
```

## CLI Commands

Grit provides a mix of plumbing and porcelain operations:

### Porcelain (User-Facing)
- `grit init`: Initialize a new repository (creates `.grit/`).
- `grit add <files...>`: Add file contents to the index. Supports glob patterns and respects `.gritignore`.
- `grit status`: Show the working tree status, hiding files ignored by `.gritignore`.
- `grit commit -m <msg>`: Create a new commit containing the current contents of the index.
- `grit log`: Show commit logs.
- `grit reset [--soft|--mixed|--hard] <commit>`: Reset current HEAD to the specified state.
- `grit diff <commit_a> <commit_b> [--stat]`: Show changes between two commits.

### Plumbing (Low-Level)
- `grit hash-object <file>`: Store file as blob, print SHA-1.
- `grit cat-file <hash>`: Display object content (blob raw, tree/commit pretty-printed).
- `grit write-tree`: Create a tree object from the current index.
- `grit checkout <hash>`: Restore working directory from tree/commit.

## Dependencies (Cargo.toml)

- **Core**: `clap` (CLI parsing), `flate2` (zlib compression), `sha1` (hashing), `hex` (hex encoding).
- **FileSystem**: `glob` (pattern matching).
- **Performance**: `rayon` (parallelism), `lru` (caching), `lazy_static` (statics).
- **Dev**: `tempfile`, `proptest` (testing), `criterion` (benchmarking).

## Extending Grit

### Adding a New Command
1. Add to `Commands` enum in `main.rs`.
2. Implement in `main()` match arm.
3. If it's a high-level user command, place in `src/commands/`.
4. If it's a low-level operation, place in `src/plumbing/`.
5. Use `GritError` for errors; add tests.

### Future Features
- Branch management (`branch`, `checkout -b`).
- Merging strategies.
- Remote operations (`fetch`, `push`, `pull`).
- Packfile support.

**Guidelines**: Maintain performance (caching/parallelism), ensure Git compatibility (especially Index and Object format), add docs/tests.
