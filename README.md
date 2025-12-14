# Grit

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/huangsam/grit/ci.yml)](https://github.com/huangsam/grit/actions)
[![License](https://img.shields.io/github/license/huangsam/grit)](https://github.com/huangsam/grit/blob/main/LICENSE)

A high-performance Git implementation in Rust with both plumbing and porcelain commands.

## Features

- **Hybrid Architecture**: Implements plumbing operations and porcelain commands
- **High Performance**: Aggressive caching and parallel processing for 2-3x faster operations
- **Git Compatible**: Full compatibility with standard Git repositories and formats
- **Ignore Support**: Respects `.gritignore` files for excluding files from staging and status

## Why Grit?

Grit is designed for speed. By leveraging Rust's zero-cost abstractions, parallel processing with Rayon, and aggressive LRU caching, Grit significantly outperforms standard Git in micro-benchmarks for small to medium repositories.

### Performance Benchmarks

| Command | Grit Time | Git Time | Speedup |
| :--- | :--- | :--- | :--- |
| `init` | ~2.1 ms | ~8.6 ms | **~4.1x** |
| `add` | ~4.1 ms | ~14.0 ms | **~3.4x** |
| `status` | ~5.4 ms | ~17.8 ms | **~3.3x** |
| `commit` | ~6.0 ms | ~25.2 ms | **~4.2x** |

*Benchmarks run on macOS with a local temporary repository.*

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialize a repository
grit init

# Stage and commit files
grit add .
grit status
grit commit -m "Initial commit"

# View history
grit log --oneline
```

### Commands

#### Porcelain (User-Friendly)

- `grit init` - Initialize repository
- `grit add <files>` - Stage files for commit
- `grit status` - Show working directory status
- `grit commit -m <msg>` - Create commit
- `grit log` - Show commit history
- `grit reset` - Reset to previous state
- `grit diff` - Show changes between commits

#### Plumbing (Low-Level)

- `grit hash-object <file>` - Store file in object database
- `grit cat-file -p <hash>` - Display object content
- `grit write-tree` - Create tree from index
- `grit checkout <hash>` - Restore working directory

### Ignoring Files

Grit supports `.gritignore` files to exclude files from staging and status output. Create a `.gritignore` file in your repository root:

```text
# Ignore temporary files
*.tmp
*.log

# Ignore build directories
build/
target/

# Ignore specific files
secret.txt
```

Patterns support:

- `*.ext` - Match files with specific extensions
- `dir/` - Ignore directories and their contents
- `file.txt` - Exact file matches

## Documentation

To learn more about Grit's architecture and API, generate and view the documentation with:

```bash
cargo doc --open
```
