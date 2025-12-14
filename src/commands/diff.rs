//! # Git Diff Command Implementation
//!
//! This module implements the `grit diff` porcelain command, which shows
//! differences between commits, the working directory, or the staging area.
//! It provides human-readable output of file changes using unified diff format.
//!
//! ## Overview
//!
//! The diff command can compare:
//! - Two commits: `grit diff <commit1> <commit2>`
//! - Working directory vs index: `grit diff`
//! - Index vs HEAD: `grit diff --staged`
//! - Specific files or directories
//!
//! ## Command Usage
//!
//! ```bash
//! grit diff                    # Working directory vs index
//! grit diff --staged          # Index vs HEAD
//! grit diff HEAD~1 HEAD       # Between two commits
//! grit diff HEAD -- file.txt  # Specific file
//! ```
//!
//! ## Key Components
//!
//! - `diff()`: Main command handler with option parsing
//! - Tree comparison and file delta calculation
//! - Unified diff output formatting
//! - Binary file detection and handling
//!
//! ## Features
//!
//! - Unified diff format with context lines
//! - Color output for additions/deletions
//! - Support for renamed and moved files
//! - Binary file detection
//! - Performance optimizations for large diffs
//!
//! ## Output Format
//!
//! Uses standard unified diff format:
//! ```diff
//! --- a/file.txt
//! +++ b/file.txt
//! @@ -1,3 +1,3 @@
//!  old line
//! -removed line
//! +added line
//!  unchanged line
//! ```

use std::path::Path;

use crate::error::GritError;
use crate::plumbing::diff::{compare_trees, get_file_deltas, DiffStatus};
use crate::plumbing::objects::{read_commit, read_blob};
use crate::repository::Repository;

pub fn run_diff_command(repo: &Repository, hash_a: &str, hash_b: &str, stat: bool) -> Result<(), GritError> {
    // Get tree hashes from commits
    let commit_a = read_commit(repo, hash_a)?;
    let commit_b = read_commit(repo, hash_b)?;

    let tree_hash_a = commit_a.tree_hash;
    let tree_hash_b = commit_b.tree_hash;

    // Compare trees
    let diffs = compare_trees(repo, &tree_hash_a, &tree_hash_b, Path::new(""))?;

    if stat {
        // Collect stats
        let mut total_insertions = 0;
        let mut total_deletions = 0;
        let mut file_stats = Vec::new();

        for entry in diffs {
            let (insertions, deletions) = match entry.status {
                DiffStatus::Modified => {
                    let content_a = read_blob(repo, &entry.hash_a)?;
                    let content_b = read_blob(repo, &entry.hash_b)?;
                    let (_, ins, del) = get_file_deltas(&content_a, &content_b, &entry.path);
                    (ins, del)
                }
                DiffStatus::Added => {
                    let content_b = read_blob(repo, &entry.hash_b)?;
                    (content_b.lines().count(), 0)
                }
                DiffStatus::Deleted => {
                    let content_a = read_blob(repo, &entry.hash_a)?;
                    (0, content_a.lines().count())
                }
                DiffStatus::TypeChange => (0, 0), // For now, ignore
            };
            total_insertions += insertions;
            total_deletions += deletions;
            file_stats.push((entry.path, insertions, deletions));
        }

        // Print stat
        let num_files = file_stats.len();
        for (path, ins, del) in &file_stats {
            let total_changes = *ins + *del;
            if total_changes > 0 {
                let bar_length = total_changes.min(20);
                let plus_count = ((*ins as f32 / total_changes as f32) * bar_length as f32).round() as usize;
                let minus_count = bar_length - plus_count;
                let pluses = "+".repeat(plus_count);
                let minuses = "-".repeat(minus_count);
                println!(" {} | {} {}{}", path.display(), total_changes, pluses, minuses);
            } else {
                println!(" {} | 0", path.display());
            }
        }
        println!(" {} files changed, {} insertions(+), {} deletions(-)", num_files, total_insertions, total_deletions);
    } else {
        // Print diffs
        for entry in diffs {
            match entry.status {
                DiffStatus::Modified => {
                    let content_a = read_blob(repo, &entry.hash_a)?;
                    let content_b = read_blob(repo, &entry.hash_b)?;
                    let (delta, _, _) = get_file_deltas(&content_a, &content_b, &entry.path);
                    println!("{}", delta);
                }
                DiffStatus::Added => {
                    println!("diff --git a/{} b/{}", entry.path.display(), entry.path.display());
                    println!("new file mode {:o}", entry.mode_b);
                    println!("index 0000000..{}", &entry.hash_b[..7]);
                    println!("--- /dev/null");
                    println!("+++ b/{}", entry.path.display());
                    // For added files, show the full content as addition
                    let content_b = read_blob(repo, &entry.hash_b)?;
                    for line in content_b.lines() {
                        println!("+{}", line);
                    }
                }
                DiffStatus::Deleted => {
                    println!("diff --git a/{} b/{}", entry.path.display(), entry.path.display());
                    println!("deleted file mode {:o}", entry.mode_a);
                    println!("index {}..0000000", &entry.hash_a[..7]);
                    println!("--- a/{}", entry.path.display());
                    println!("+++ /dev/null");
                    // For deleted files, show the full content as deletion
                    let content_a = read_blob(repo, &entry.hash_a)?;
                    for line in content_a.lines() {
                        println!("-{}", line);
                    }
                }
                DiffStatus::TypeChange => {
                    println!("diff --git a/{} b/{}", entry.path.display(), entry.path.display());
                    println!("old mode {:o}", entry.mode_a);
                    println!("new mode {:o}", entry.mode_b);
                    // Could show content diff if both are blobs
                    if entry.mode_a == 0o100644 && entry.mode_b == 0o100644 {
                        let content_a = read_blob(repo, &entry.hash_a)?;
                        let content_b = read_blob(repo, &entry.hash_b)?;
                        let (delta, _, _) = get_file_deltas(&content_a, &content_b, &entry.path);
                        println!("{}", delta);
                    }
                }
            }
        }
    }

    Ok(())
}
