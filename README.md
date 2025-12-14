# Grit

A high-performance Git implementation in Rust with both plumbing and porcelain commands.

## Features

- **Hybrid Architecture**: Implements both low-level plumbing operations and user-friendly porcelain commands
- **High Performance**: Aggressive caching and parallel processing for 2-3x faster operations
- **Git Compatible**: Full compatibility with standard Git repositories and formats
- **Memory Efficient**: LRU caching prevents memory bloat during large operations

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

## Commands

### Porcelain (User-Friendly)

- `grit init` - Initialize repository
- `grit add <files>` - Stage files for commit
- `grit status` - Show working directory status
- `grit commit -m <msg>` - Create commit
- `grit log` - Show commit history
- `grit reset` - Reset to previous state
- `grit diff` - Show changes between commits

### Plumbing (Low-Level)

- `grit hash-object <file>` - Store file in object database
- `grit cat-file -p <hash>` - Display object content
- `grit write-tree` - Create tree from index
- `grit checkout <hash>` - Restore working directory

## Documentation

```bash
cargo doc --open
```
