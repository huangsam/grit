//! # Git Reset Command Implementation
//!
//! This module implements the `grit reset` porcelain command, which moves the
//! current HEAD to a specified commit and optionally updates the index and
//! working directory. It provides flexible ways to undo commits or staging.
//!
//! ## Overview
//!
//! The reset command supports three modes:
//! - `--soft`: Move HEAD only (keep index and working directory)
//! - `--mixed` (default): Move HEAD and index (keep working directory)
//! - `--hard`: Move HEAD, index, and working directory
//!
//! ## Command Usage
//!
//! ```bash
//! grit reset --soft HEAD~1    # Undo last commit, keep changes staged
//! grit reset HEAD~1           # Undo last commit, keep changes unstaged
//! grit reset --hard HEAD~1    # Undo last commit, discard all changes
//! ```
//!
//! ## Key Components
//!
//! - `reset()`: Main function handling reset logic
//! - Mode-specific reset implementations
//! - HEAD and index updating
//! - Working directory restoration
//! - Safety checks and confirmations
//!
//! ## Features
//!
//! - Support for all three reset modes
//! - Commit hash resolution (HEAD~n, branch names)
//! - Preservation of uncommitted changes in soft/mixed modes
//! - Integration with checkout for working directory updates
//!
//! ## Safety
//!
//! - Validation of target commits
//! - Warning for destructive operations
//! - Backup recommendations for hard resets

use crate::error::GritError;
use crate::plumbing::commits::update_ref;
use crate::plumbing::index::{Index, IndexEntry, write_index};
use crate::plumbing::objects::{read_object, ObjectType};
use crate::plumbing::checkout::{restore_snapshot, parse_tree_entries};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Copy)]
/// Reset modes for the `grit reset` command
///
/// Determines how much of the repository state to reset when moving HEAD.
/// Similar to `git reset --soft|--mixed|--hard`.
pub enum ResetMode {
    /// Move HEAD only, keep index and working directory unchanged
    ///
    /// Useful for amending commits without changing staged changes.
    Soft,
    /// Move HEAD and update index, keep working directory unchanged
    ///
    /// Default behavior - unstages changes but preserves working directory modifications.
    Mixed,
    /// Move HEAD, update index, and restore working directory
    ///
    /// Completely resets repository to target commit state. **Destructive operation**.
    Hard,
}

/// Reset the current HEAD to a specified commit
///
/// Moves the current branch tip to the specified commit and optionally updates
/// the index and working directory according to the reset mode.
///
/// # Arguments
///
/// * `commit_hash` - The hash of the commit to reset to
/// * `mode` - The reset mode determining what to update (Soft, Mixed, or Hard)
/// * `repo_root` - The root directory of the Git repository
///
/// # Returns
///
/// Returns `Ok(())` if the reset operation completed successfully.
///
/// # Errors
///
/// Returns `GritError` if:
/// - The commit hash doesn't exist or isn't a commit object
/// - Repository refs cannot be updated
/// - Index cannot be read or written
/// - Working directory files cannot be restored (for hard reset)
///
/// # Reset Modes
///
/// - **Soft**: Only moves HEAD, preserves index and working directory
/// - **Mixed**: Moves HEAD and updates index, preserves working directory
/// - **Hard**: Moves HEAD, updates index, and restores working directory
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
/// use grit::commands::reset::{reset, ResetMode};
///
/// let repo_root = Path::new("/path/to/repo");
///
/// // Soft reset to previous commit
/// reset("abc123...", ResetMode::Soft, repo_root).unwrap();
///
/// // Hard reset (destructive!)
/// reset("def456...", ResetMode::Hard, repo_root).unwrap();
/// ```
pub fn reset(commit_hash: &str, mode: ResetMode, repo_root: &Path) -> Result<(), GritError> {
    // 1. Validate commit exists
    let object = read_object(commit_hash, repo_root)?;
    if object.obj_type != ObjectType::Commit {
        return Err(GritError::RepositoryError(format!("{} is not a commit", commit_hash)));
    }

    // Read old index for hard reset cleanup
    let old_index = if let ResetMode::Hard = mode {
        Some(crate::plumbing::index::read_index(repo_root)?)
    } else {
        None
    };

    // 2. Update HEAD
    let head_path = repo_root.join(".grit").join("HEAD");
    let head_content = fs::read_to_string(&head_path)?;
    let ref_name = if let Some(ref_name) = head_content.strip_prefix("ref: ") {
        ref_name.trim().to_string()
    } else {
        "HEAD".to_string()
    };

    update_ref(&ref_name, commit_hash, repo_root)?;

    if let ResetMode::Soft = mode {
        return Ok(());
    }

    // 3. Update Index (Mixed and Hard)
    let commit_content = String::from_utf8_lossy(&object.content);
    let tree_hash = extract_tree_hash(&commit_content)?;

    let new_index = build_index_from_tree(&tree_hash, repo_root)?;
    write_index(&new_index, repo_root)?;

    if let ResetMode::Mixed = mode {
        return Ok(());
    }

    // 4. Update Working Directory (Hard)
    if let ResetMode::Hard = mode {
        // Delete files that are in old index but not in new index
        if let Some(old) = old_index {
            for entry in &old.entries {
                // Check if path exists in new index
                if !new_index.entries.iter().any(|e| e.path == entry.path) {
                    // Delete file
                    let path = repo_root.join(&entry.path);
                    if path.exists() {
                        fs::remove_file(path)?;
                    }
                }
            }
        }

        restore_snapshot(commit_hash, repo_root)?;
    }

    Ok(())
}

/// Reset specific paths to match a target commit
///
/// Updates the index entries for the specified paths to match the state
/// of those paths in the target commit. This is similar to `git reset <commit> -- <paths>`.
///
/// # Arguments
///
/// * `commit_hash` - The hash of the commit containing the desired state
/// * `paths` - Slice of file/directory paths to reset
/// * `repo_root` - The root directory of the Git repository
///
/// # Returns
///
/// Returns `Ok(())` if the paths were successfully reset in the index.
///
/// # Errors
///
/// Returns `GritError` if:
/// - The commit hash doesn't exist or isn't a commit object
/// - Index cannot be read or written
/// - Tree objects cannot be parsed
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
/// use grit::commands::reset::reset_paths;
///
/// let repo_root = Path::new("/path/to/repo");
///
/// // Reset specific files to match HEAD~1
/// reset_paths("abc123...", &["src/main.rs".to_string(), "Cargo.toml".to_string()], repo_root).unwrap();
/// ```
pub fn reset_paths(commit_hash: &str, paths: &[String], repo_root: &Path) -> Result<(), GritError> {
    // 1. Get tree from commit
    let object = read_object(commit_hash, repo_root)?;
    if object.obj_type != ObjectType::Commit {
        return Err(GritError::RepositoryError(format!("{} is not a commit", commit_hash)));
    }
    let commit_content = String::from_utf8_lossy(&object.content);
    let tree_hash = extract_tree_hash(&commit_content)?;

    // 2. Read current index
    let mut index = crate::plumbing::index::read_index(repo_root)?;

    // 3. Build index from tree to get target state
    let tree_index = build_index_from_tree(&tree_hash, repo_root)?;

    for path in paths {
        // 1. Remove all entries in `index` matching `path`
        index.entries.retain(|e| !matches_path(&e.path, path));

        // 2. Add all entries from `tree_index` matching `path`
        for entry in &tree_index.entries {
            if matches_path(&entry.path, path) {
                index.add_entry(entry.clone());
            }
        }
    }

    write_index(&index, repo_root)?;
    Ok(())
}

fn matches_path(entry_path: &str, target_path: &str) -> bool {
    entry_path == target_path || entry_path.starts_with(&format!("{}/", target_path))
}

fn extract_tree_hash(commit_content: &str) -> Result<String, GritError> {
    commit_content.lines()
        .find(|line| line.starts_with("tree "))
        .map(|line| line[5..].to_string())
        .ok_or_else(|| GritError::CorruptObject("Commit missing tree hash".to_string()))
}

fn build_index_from_tree(tree_hash: &str, repo_root: &Path) -> Result<Index, GritError> {
    let mut index = Index::new();
    collect_index_entries(tree_hash, Path::new(""), &mut index, repo_root)?;
    Ok(index)
}

fn collect_index_entries(tree_hash: &str, current_path: &Path, index: &mut Index, repo_root: &Path) -> Result<(), GritError> {
    let tree_obj = read_object(tree_hash, repo_root)?;
    let entries = parse_tree_entries(&tree_obj.content)?;

    for entry in entries {
        let entry_path = current_path.join(&entry.name);

        if entry.mode == "40000" {
            // Directory
            let sub_tree_hash = hex::encode(entry.hash);
            collect_index_entries(&sub_tree_hash, &entry_path, index, repo_root)?;
        } else {
            // File
            // Read blob to get size
            let blob_hash = hex::encode(entry.hash);
            let blob_obj = read_object(&blob_hash, repo_root)?;
            let size = blob_obj.content.len() as u32;

            let index_entry = IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode: u32::from_str_radix(&entry.mode, 8).unwrap_or(0o100644),
                uid: 0,
                gid: 0,
                size,
                hash: entry.hash,
                flags: 0,
                path: entry_path.to_string_lossy().to_string(),
            };
            index.add_entry(index_entry);
        }
    }
    Ok(())
}
