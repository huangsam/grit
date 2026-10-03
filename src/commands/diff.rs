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
//! - Working directory vs commit: `grit diff <commit>`
//! - Working directory vs index: `grit diff`
//! - Index vs HEAD: `grit diff --staged`
//!
//! ## Command Usage
//!
//! ```bash
//! grit diff                    # Working directory vs index
//! grit diff --staged          # Index vs HEAD
//! grit diff HEAD              # Working directory vs HEAD
//! grit diff commit_a commit_b # Between two commits
//! ```
//!
//! ## Output Format
//!
//! Uses standard unified diff format:
//! ```diff
//! diff --git a/file.txt b/file.txt
//! index 0000000..abcdef1 100644
//! --- a/file.txt
//! +++ b/file.txt
//! @@ -1,3 +1,3 @@
//!  old line
//! -removed line
//! +added line
//!  unchanged line
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::error::GritError;
use crate::plumbing::commits::get_current_commit;
use crate::plumbing::diff::{DiffStatus, compare_trees, get_file_deltas};
use crate::plumbing::index::{Index, read_index};
use crate::plumbing::objects::{ObjectType, read_blob, read_commit, read_object};
use crate::plumbing::trees::{build_index_from_tree, write_tree_from_index};
use crate::repository::Repository;

/// Represents a single file difference prepared for display or statistical reporting.
#[derive(Debug, Clone)]
pub struct DiffItem {
    pub path: PathBuf,
    pub status: DiffStatus,
    pub mode_a: u32,
    pub mode_b: u32,
    pub hash_a: String,
    pub hash_b: String,
    pub content_a: String,
    pub content_b: String,
}

/// Execute the diff command across commits, working tree, and index.
///
/// # Arguments
///
/// * `repo` - The repository to operate on
/// * `commit_a` - Optional first commit or revision
/// * `commit_b` - Optional second commit or revision
/// * `staged` - If true, compare index against HEAD or `commit_a`
/// * `stat` - If true, show diffstat summary instead of patch
pub fn diff(
    repo: &Repository,
    commit_a: Option<&str>,
    commit_b: Option<&str>,
    staged: bool,
    stat: bool,
) -> Result<(), GritError> {
    let items = if staged {
        if commit_b.is_some() {
            return Err(GritError::repo("Cannot use --staged with two commits"));
        }
        diff_staged(repo, commit_a)?
    } else {
        match (commit_a, commit_b) {
            (None, None) => diff_working_tree_vs_index(repo)?,
            (Some(rev), None) => diff_commit_vs_working_tree(repo, rev)?,
            (Some(rev_a), Some(rev_b)) => {
                let tree_a = resolve_to_tree_hash(repo, rev_a)?;
                let tree_b = resolve_to_tree_hash(repo, rev_b)?;
                diff_trees(repo, &tree_a, &tree_b)?
            }
            (None, Some(_)) => unreachable!(),
        }
    };

    print_diff_items(&items, stat);
    Ok(())
}

/// Execute the diff command to show changes between two commits
///
/// Maintained for backwards compatibility with previous plumbing calls.
pub fn run_diff_command(
    repo: &Repository,
    hash_a: &str,
    hash_b: &str,
    stat: bool,
) -> Result<(), GritError> {
    diff(repo, Some(hash_a), Some(hash_b), false, stat)
}

/// Resolves a revision string (branch name, HEAD, commit hash, or tree hash) to a tree hash.
pub fn resolve_to_tree_hash(repo: &Repository, rev: &str) -> Result<String, GritError> {
    let obj_hash = resolve_object_hash(&repo.root, rev)?;
    let obj = read_object(&obj_hash, &repo.root)?;
    match obj.obj_type {
        ObjectType::Commit => {
            let commit = read_commit(repo, &obj_hash)?;
            Ok(commit.tree_hash)
        }
        ObjectType::Tree => Ok(obj_hash),
        ObjectType::Blob => Err(GritError::repo(format!(
            "Object '{}' is a blob, expected commit or tree",
            rev
        ))),
    }
}

/// Resolves a revision specifier to an object hash in the repository.
fn resolve_object_hash(repo_root: &Path, rev: &str) -> Result<String, GritError> {
    if rev == "HEAD" {
        return get_current_commit(repo_root)?.ok_or_else(GritError::no_commits);
    }

    // Branch ref: .grit/refs/heads/<rev>
    let branch_ref = repo_root.join(".grit").join("refs").join("heads").join(rev);
    if branch_ref.is_file() {
        let content = fs::read_to_string(&branch_ref)?;
        return Ok(content.trim().to_string());
    }

    // Generic ref: .grit/<rev>
    let generic_ref = repo_root.join(".grit").join(rev);
    if generic_ref.is_file() {
        let content = fs::read_to_string(&generic_ref)?;
        return Ok(content.trim().to_string());
    }

    // Full 40-character hash
    if rev.len() == 40 && hex::decode(rev).is_ok() {
        let (prefix, suffix) = rev.split_at(2);
        let obj_path = repo_root
            .join(".grit")
            .join("objects")
            .join(prefix)
            .join(suffix);
        if obj_path.is_file() {
            return Ok(rev.to_string());
        }
    }

    // Short hex hash prefix (e.g. 7 characters)
    if rev.len() >= 4 && rev.len() < 40 && rev.chars().all(|c| c.is_ascii_hexdigit()) {
        let (prefix, suffix) = rev.split_at(2);
        let obj_dir = repo_root.join(".grit").join("objects").join(prefix);
        if obj_dir.is_dir() {
            let mut matches = Vec::new();
            for entry in fs::read_dir(obj_dir)? {
                let entry = entry?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with(suffix) {
                    matches.push(format!("{}{}", prefix, file_name));
                }
            }
            if matches.len() == 1 {
                return Ok(matches.remove(0));
            } else if matches.len() > 1 {
                return Err(GritError::repo(format!(
                    "short revision '{}' is ambiguous",
                    rev
                )));
            }
        }
    }

    Err(GritError::repo(format!(
        "unknown revision or path not in the working tree: '{}'",
        rev
    )))
}

/// Compute SHA-1 blob hash for raw bytes.
fn compute_blob_hash(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Compares working tree files against the staging index.
fn diff_working_tree_vs_index(repo: &Repository) -> Result<Vec<DiffItem>, GritError> {
    let index = match read_index(&repo.root) {
        Ok(idx) => idx,
        Err(_) => return Ok(Vec::new()),
    };

    let mut items = Vec::new();

    for entry in &index.entries {
        let file_path = repo.root.join(&entry.path);
        let path = PathBuf::from(&entry.path);
        let staged_hash = hex::encode(entry.hash);
        let staged_content = read_blob(repo, &staged_hash)?;

        if file_path.is_file() {
            let working_bytes = fs::read(&file_path)?;
            let working_content = String::from_utf8_lossy(&working_bytes).to_string();

            if working_content != staged_content {
                let working_hash = compute_blob_hash(&working_bytes);
                items.push(DiffItem {
                    path,
                    status: DiffStatus::Modified,
                    mode_a: entry.mode,
                    mode_b: entry.mode,
                    hash_a: staged_hash,
                    hash_b: working_hash,
                    content_a: staged_content,
                    content_b: working_content,
                });
            }
        } else {
            items.push(DiffItem {
                path,
                status: DiffStatus::Deleted,
                mode_a: entry.mode,
                mode_b: 0,
                hash_a: staged_hash,
                hash_b: String::new(),
                content_a: staged_content,
                content_b: String::new(),
            });
        }
    }

    Ok(items)
}

/// Compares working tree files against a specific commit or revision.
fn diff_commit_vs_working_tree(repo: &Repository, rev: &str) -> Result<Vec<DiffItem>, GritError> {
    let tree_hash = resolve_to_tree_hash(repo, rev)?;
    let commit_index = build_index_from_tree(&tree_hash, &repo.root)?;
    let current_index = read_index(&repo.root).unwrap_or_else(|_| Index::new());

    let mut items = Vec::new();

    for entry in &commit_index.entries {
        let file_path = repo.root.join(&entry.path);
        let path = PathBuf::from(&entry.path);
        let commit_hash = hex::encode(entry.hash);
        let commit_content = read_blob(repo, &commit_hash)?;

        if file_path.is_file() {
            let working_bytes = fs::read(&file_path)?;
            let working_content = String::from_utf8_lossy(&working_bytes).to_string();

            if working_content != commit_content {
                let working_hash = compute_blob_hash(&working_bytes);
                items.push(DiffItem {
                    path,
                    status: DiffStatus::Modified,
                    mode_a: entry.mode,
                    mode_b: entry.mode,
                    hash_a: commit_hash,
                    hash_b: working_hash,
                    content_a: commit_content,
                    content_b: working_content,
                });
            }
        } else {
            items.push(DiffItem {
                path,
                status: DiffStatus::Deleted,
                mode_a: entry.mode,
                mode_b: 0,
                hash_a: commit_hash,
                hash_b: String::new(),
                content_a: commit_content,
                content_b: String::new(),
            });
        }
    }

    // Check files tracked in current index that were absent in commit
    for entry in &current_index.entries {
        if !commit_index.entries.iter().any(|e| e.path == entry.path) {
            let file_path = repo.root.join(&entry.path);
            if file_path.is_file() {
                let working_bytes = fs::read(&file_path)?;
                let working_content = String::from_utf8_lossy(&working_bytes).to_string();
                let working_hash = compute_blob_hash(&working_bytes);
                items.push(DiffItem {
                    path: PathBuf::from(&entry.path),
                    status: DiffStatus::Added,
                    mode_a: 0,
                    mode_b: entry.mode,
                    hash_a: String::new(),
                    hash_b: working_hash,
                    content_a: String::new(),
                    content_b: working_content,
                });
            }
        }
    }

    items.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(items)
}

/// Compares the staging index against HEAD (or a specified revision).
fn diff_staged(repo: &Repository, commit_a: Option<&str>) -> Result<Vec<DiffItem>, GritError> {
    let index = match read_index(&repo.root) {
        Ok(idx) => idx,
        Err(_) => return Ok(Vec::new()),
    };

    let index_tree_hash = write_tree_from_index(&index, &repo.root)?;

    let tree_hash_a = if let Some(rev) = commit_a {
        resolve_to_tree_hash(repo, rev)?
    } else {
        match get_current_commit(&repo.root)? {
            Some(head_hash) => resolve_to_tree_hash(repo, &head_hash)?,
            None => {
                let empty_index = Index::new();
                write_tree_from_index(&empty_index, &repo.root)?
            }
        }
    };

    diff_trees(repo, &tree_hash_a, &index_tree_hash)
}

/// Compares two tree objects and extracts their file contents.
fn diff_trees(
    repo: &Repository,
    tree_hash_a: &str,
    tree_hash_b: &str,
) -> Result<Vec<DiffItem>, GritError> {
    let entries = compare_trees(repo, tree_hash_a, tree_hash_b, Path::new(""))?;
    let mut items = Vec::new();

    for entry in entries {
        let content_a = if !entry.hash_a.is_empty() {
            read_blob(repo, &entry.hash_a)?
        } else {
            String::new()
        };
        let content_b = if !entry.hash_b.is_empty() {
            read_blob(repo, &entry.hash_b)?
        } else {
            String::new()
        };

        items.push(DiffItem {
            path: entry.path,
            status: entry.status,
            mode_a: entry.mode_a,
            mode_b: entry.mode_b,
            hash_a: entry.hash_a,
            hash_b: entry.hash_b,
            content_a,
            content_b,
        });
    }

    Ok(items)
}

/// Print diff items either as full patch or as diffstat.
fn print_diff_items(items: &[DiffItem], stat: bool) {
    if stat {
        let mut total_insertions = 0;
        let mut total_deletions = 0;
        let mut file_stats = Vec::new();

        for item in items {
            let (ins, del) = match item.status {
                DiffStatus::Modified => {
                    let (_, ins, del) =
                        get_file_deltas(&item.content_a, &item.content_b, &item.path);
                    (ins, del)
                }
                DiffStatus::Added => (item.content_b.lines().count(), 0),
                DiffStatus::Deleted => (0, item.content_a.lines().count()),
                DiffStatus::TypeChange => (0, 0),
            };
            total_insertions += ins;
            total_deletions += del;
            file_stats.push((&item.path, ins, del));
        }

        let num_files = file_stats.len();
        if num_files > 0 {
            for (path, ins, del) in &file_stats {
                print_stat_bar(path, *ins, *del);
            }
            let files_str = if num_files == 1 { "file" } else { "files" };
            let ins_str = if total_insertions == 1 {
                "insertion(+)"
            } else {
                "insertions(+)"
            };
            let del_str = if total_deletions == 1 {
                "deletion(-)"
            } else {
                "deletions(-)"
            };
            println!(
                " {} {} changed, {} {}, {} {}",
                num_files, files_str, total_insertions, ins_str, total_deletions, del_str
            );
        }
    } else {
        for item in items {
            print_item_diff(item);
        }
    }
}

/// Print unified diff output for a single item.
fn print_item_diff(item: &DiffItem) {
    let short_hash_a = if item.hash_a.len() >= 7 {
        &item.hash_a[..7]
    } else {
        "0000000"
    };
    let short_hash_b = if item.hash_b.len() >= 7 {
        &item.hash_b[..7]
    } else {
        "0000000"
    };

    match item.status {
        DiffStatus::Modified => {
            println!(
                "diff --git a/{} b/{}",
                item.path.display(),
                item.path.display()
            );
            println!("index {}..{} {:o}", short_hash_a, short_hash_b, item.mode_b);
            let (delta, _, _) = get_file_deltas(&item.content_a, &item.content_b, &item.path);
            print!("{}", delta);
        }
        DiffStatus::Added => {
            println!(
                "diff --git a/{} b/{}",
                item.path.display(),
                item.path.display()
            );
            println!("new file mode {:o}", item.mode_b);
            println!("index 0000000..{}", short_hash_b);
            println!("--- /dev/null");
            println!("+++ b/{}", item.path.display());
            for line in item.content_b.lines() {
                println!("+{}", line);
            }
        }
        DiffStatus::Deleted => {
            println!(
                "diff --git a/{} b/{}",
                item.path.display(),
                item.path.display()
            );
            println!("deleted file mode {:o}", item.mode_a);
            println!("index {}..0000000", short_hash_a);
            println!("--- a/{}", item.path.display());
            println!("+++ /dev/null");
            for line in item.content_a.lines() {
                println!("-{}", line);
            }
        }
        DiffStatus::TypeChange => {
            println!(
                "diff --git a/{} b/{}",
                item.path.display(),
                item.path.display()
            );
            println!("old mode {:o}", item.mode_a);
            println!("new mode {:o}", item.mode_b);
            if item.mode_a == 0o100644 && item.mode_b == 0o100644 {
                let (delta, _, _) = get_file_deltas(&item.content_a, &item.content_b, &item.path);
                print!("{}", delta);
            }
        }
    }
}

/// Helper: print a compact stat bar for a file
fn print_stat_bar(path: &Path, ins: usize, del: usize) {
    let total_changes = ins + del;
    if total_changes > 0 {
        let bar_length = total_changes.min(20);
        let plus_count =
            (((ins as f32) / (total_changes as f32)) * (bar_length as f32)).round() as usize;
        let minus_count = bar_length - plus_count;
        let pluses = "+".repeat(plus_count);
        let minuses = "-".repeat(minus_count);
        println!(
            " {} | {} {}{}",
            path.display(),
            total_changes,
            pluses,
            minuses
        );
    } else {
        println!(" {} | 0", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::add::add_files;
    use crate::plumbing::commits::create_commit;
    use crate::repository::initialize_repo;
    use tempfile::TempDir;

    #[test]
    fn test_diff_working_tree_vs_index() {
        let temp_dir = TempDir::new().unwrap();
        initialize_repo(temp_dir.path()).unwrap();
        let repo = Repository::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "initial content\n").unwrap();
        add_files(&["test.txt".to_string()], temp_dir.path()).unwrap();

        // No changes yet
        let items = diff_working_tree_vs_index(&repo).unwrap();
        assert!(items.is_empty());

        // Modify file
        fs::write(&file_path, "modified content\n").unwrap();
        let items = diff_working_tree_vs_index(&repo).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, DiffStatus::Modified);
        assert_eq!(items[0].content_a, "initial content\n");
        assert_eq!(items[0].content_b, "modified content\n");
    }

    #[test]
    fn test_diff_staged() {
        let temp_dir = TempDir::new().unwrap();
        initialize_repo(temp_dir.path()).unwrap();
        let repo = Repository::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "staged content\n").unwrap();
        add_files(&["test.txt".to_string()], temp_dir.path()).unwrap();

        // Diff staged with no commits yet
        let items = diff_staged(&repo, None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, DiffStatus::Added);
    }

    #[test]
    fn test_diff_commit_vs_working_tree() {
        let temp_dir = TempDir::new().unwrap();
        initialize_repo(temp_dir.path()).unwrap();
        let repo = Repository::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "commit content\n").unwrap();
        add_files(&["test.txt".to_string()], temp_dir.path()).unwrap();

        let index = read_index(temp_dir.path()).unwrap();
        let tree_hash = write_tree_from_index(&index, temp_dir.path()).unwrap();
        let commit_hash = create_commit(&tree_hash, None, "initial", temp_dir.path()).unwrap();

        // Modify working tree
        fs::write(&file_path, "working content\n").unwrap();
        let items = diff_commit_vs_working_tree(&repo, &commit_hash).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, DiffStatus::Modified);
    }
}
