# Agent Guidelines for Grit

## Core Invariants
- **Git Format Compatibility**: Object storage (`.grit/objects/`), trees, commits, and index (`.grit/index`) must preserve byte-level Git specifications (zlib deflate/inflate, SHA-1 loose objects, DIRC v2 binary index serialization with SHA-1 trailer).
- **Plumbing / Porcelain Separation**: Low-level Git primitives belong in `src/plumbing/` (`objects`, `trees`, `commits`, `index`, `ignores`, `checkout`, `diff`). High-level CLI commands live in `src/commands/` and must delegate directly to plumbing functions rather than reimplementing Git internals.
- **Cache Thread Safety & Coherence**: The multi-tier LRU system (`GLOBAL_CACHE` via `ObjectCache`, `HashCache`, `TreeCache`) must protect state behind `Mutex` and maintain bounded capacities. Cache lookups must remain transparent and never return stale state across working directory modifications or index writes.
- **Structured Error Handling**: Recoverable failures must never panic; return `GritError` / `RepoError` variants (`GritError::repo`, `RepoError::NoCommits`, `RepoError::FileOutsideRepo`, `RepoError::InvalidIndex`, etc.) with clear operational context.
- **Ignore Enforcement**: Working tree traversals and staging operations (`grit add`, `grit status`) must evaluate `.gritignore` patterns via `src/plumbing/ignores.rs` before scanning or modifying index entries.

## Build & Quality Checks
Run before completing tasks:
```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
cargo bench                    # in benches/
```

## Performance Expectations
- `grit init`: ~2 ms (~4x faster than Git on small/medium repos).
- `grit add`: ~4 ms (~3x faster than Git).
- `grit status`: ~5 ms (~3x faster than Git).
- `grit commit`: ~6 ms (~4x faster than Git).
- Cache memory footprint: bounded by default capacities (1,000 objects, 5,000 hashes, 2,000 trees).
- Changes must not regress Criterion benchmarks.
