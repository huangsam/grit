# Grit - AI Development Guide

## Project Overview

Grit is a high-performance, from-scratch implementation of Git's core operations in Rust. While originally focused on low-level plumbing primitives (objects, trees, commits), it has evolved to include essential porcelain commands (`add`, `status`, `reset`). It features aggressive optimizations like LRU caching, parallel processing, and buffered I/O.

## Architecture

Grit follows Git's object model with a modular design:

- **Objects**: Blobs (file content), Trees (directory structure), Commits (history snapshots).
- **Index**: Fully compatible Git Index (staging area) implementation.
- **Caching**: Multi-layer LRU system for hashes, decompressed objects, and parsed trees.

## CLI Commands

Grit provides a mix of plumbing and porcelain operations:

### Porcelain (User-Facing)
- `grit init`: Initialize a new repository (creates `.grit/`).
- `grit add <files...>`: Add file contents to the index. Supports glob patterns and respects `.gritignore`.
- `grit status`: Show the working tree status, hiding files ignored by `.gritignore`.
- `grit commit -m <msg>`: Create a new commit containing the current contents of the index.
- `grit log [-n <count>] [--oneline] [<commit>]`: Show commit logs.
- `grit checkout [-b] <branch|hash>`: Restore working directory or switch branches.
- `grit reset [--soft|--mixed|--hard] <commit>`: Reset current HEAD to the specified state.
- `grit diff [<commit_a>] [<commit_b>] [--staged|--cached] [--stat]`: Show changes between commits, commit and working tree, working tree and index, or staged changes.
- `grit branch [-d] [<name>]`: List, create, or delete branches.

### Plumbing (Low-Level)
- `grit hash-object <file>`: Store file as blob, print SHA-1.
- `grit cat-file <hash>`: Display object content (blob raw, tree/commit pretty-printed).
- `grit write-tree`: Create a tree object from the current index.

## Extending Grit

### Adding a New Command
1. Add to `Commands` enum in `main.rs`.
2. Implement in `main()` match arm.
3. If it's a high-level user command, place in `src/commands/`.
4. If it's a low-level operation, place in `src/plumbing/`.
5. Use `GritError` and/or helpers (e.g., `GritError::repo`, `GritError::file_outside_repo`).
6. Add a short doc comment and a focused unit test for any new error case.

### Future Features
- Branch management (`branch`, `checkout -b`).
- Merging strategies.
- Remote operations (`fetch`, `push`, `pull`).
- Packfile support.

**Guidelines**: Maintain performance (caching/parallelism), ensure Git compatibility (especially Index and Object format), add docs/tests.
