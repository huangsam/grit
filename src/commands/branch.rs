//! # Git Branch Command Implementation
//!
//! This module implements the `grit branch` porcelain command, which manages
//! branches in a repository. It allows listing existing branches, creating
//! new branches pointing to the current commit, and deleting branches.
//!
//! ## Overview
//!
//! Branches in Git are lightweight, movable pointers to commits. In Grit,
//! local branches are stored as files in `.grit/refs/heads/`, where each file
//! contains the 40-character hex SHA-1 hash of the branch tip commit.
//!
//! ## Command Usage
//!
//! ```bash
//! grit branch              # List all local branches
//! grit branch <name>       # Create a new branch pointing to current HEAD
//! grit branch -d <name>    # Delete a branch
//! ```

use crate::error::GritError;
use crate::plumbing::commits::{get_current_commit, validate_ref_name};
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a local branch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    /// The name of the branch (e.g., "main", "feature/login")
    pub name: String,
    /// Whether this is the currently checked-out branch
    pub is_current: bool,
    /// The target commit hash pointed to by this branch
    pub commit_hash: String,
}

/// Recursively collects all branch names and their paths under a directory.
fn collect_branch_files(
    dir: &Path,
    prefix: &str,
    files: &mut Vec<(String, PathBuf)>,
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if path.is_dir() {
            let next_prefix = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };
            collect_branch_files(&path, &next_prefix, files)?;
        } else if path.is_file() {
            let branch_name = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };
            files.push((branch_name, path));
        }
    }

    Ok(())
}

/// Lists all local branches in the repository.
///
/// Returns a sorted list of `BranchInfo` describing each branch and whether
/// it is currently checked out according to `.grit/HEAD`.
pub fn list_branches(repo_root: &Path) -> Result<Vec<BranchInfo>, GritError> {
    let heads_dir = repo_root.join(".grit").join("refs").join("heads");
    let head_path = repo_root.join(".grit").join("HEAD");

    let current_branch_name = if head_path.exists() {
        let head_content = fs::read_to_string(&head_path)?;
        let head_content = head_content.trim();
        head_content
            .strip_prefix("ref: refs/heads/")
            .map(|s| s.to_string())
    } else {
        None
    };

    let mut branch_files = Vec::new();
    collect_branch_files(&heads_dir, "", &mut branch_files)?;

    let mut branches = Vec::new();
    for (name, path) in branch_files {
        let commit_hash = fs::read_to_string(&path)?.trim().to_string();
        let is_current = current_branch_name.as_deref() == Some(&name);
        branches.push(BranchInfo {
            name,
            is_current,
            commit_hash,
        });
    }

    branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(branches)
}

/// Creates a new branch pointing to the current HEAD commit.
pub fn create_branch(branch_name: &str, repo_root: &Path) -> Result<(), GritError> {
    validate_ref_name(branch_name)?;

    let current_commit = get_current_commit(repo_root)?
        .ok_or_else(|| GritError::repo("Cannot create branch: not a valid object name: 'HEAD'"))?;

    let branch_path = repo_root
        .join(".grit")
        .join("refs")
        .join("heads")
        .join(branch_name);

    if branch_path.exists() {
        return Err(GritError::repo(format!(
            "A branch named '{}' already exists",
            branch_name
        )));
    }

    if let Some(parent) = branch_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&branch_path, format!("{}\n", current_commit))?;
    Ok(())
}

/// Deletes an existing local branch.
///
/// Returns the commit hash that the deleted branch was pointing to.
pub fn delete_branch(branch_name: &str, repo_root: &Path) -> Result<String, GritError> {
    validate_ref_name(branch_name)?;

    let head_path = repo_root.join(".grit").join("HEAD");
    if head_path.exists() {
        let head_content = fs::read_to_string(&head_path)?;
        let head_content = head_content.trim();
        if let Some(current_name) = head_content.strip_prefix("ref: refs/heads/")
            && current_name == branch_name
        {
            return Err(GritError::repo(format!(
                "Cannot delete branch '{}' checked out at '{}'",
                branch_name,
                repo_root.display()
            )));
        }
    }

    let branch_path = repo_root
        .join(".grit")
        .join("refs")
        .join("heads")
        .join(branch_name);

    if !branch_path.exists() {
        return Err(GritError::repo(format!(
            "branch '{}' not found",
            branch_name
        )));
    }

    let commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
    fs::remove_file(branch_path)?;

    Ok(commit_hash)
}

/// Runs the branch porcelain command.
pub fn run_branch_command(
    name: Option<&str>,
    delete: Option<&str>,
    repo_root: &Path,
) -> Result<(), GritError> {
    if let Some(del_name) = delete {
        let hash = delete_branch(del_name, repo_root)?;
        let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
        println!("Deleted branch {} (was {}).", del_name, short_hash);
    } else if let Some(new_name) = name {
        create_branch(new_name, repo_root)?;
    } else {
        let branches = list_branches(repo_root)?;
        for branch in branches {
            if branch.is_current {
                println!("* {}", branch.name);
            } else {
                println!("  {}", branch.name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plumbing::commits::{create_commit, update_ref};
    use crate::repository::initialize_repo;
    use tempfile::TempDir;

    fn setup_repo_with_commit() -> (TempDir, String) {
        let dir = TempDir::new().unwrap();
        initialize_repo(dir.path()).unwrap();

        // Create an initial commit
        let commit_hash = create_commit(
            "0000000000000000000000000000000000000000",
            None,
            "Initial commit",
            dir.path(),
        )
        .unwrap();

        update_ref("refs/heads/main", &commit_hash, dir.path()).unwrap();
        (dir, commit_hash)
    }

    #[test]
    fn test_list_branches_single_branch() {
        let (dir, commit_hash) = setup_repo_with_commit();
        let branches = list_branches(dir.path()).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
        assert_eq!(branches[0].commit_hash, commit_hash);
    }

    #[test]
    fn test_create_and_delete_branch() {
        let (dir, commit_hash) = setup_repo_with_commit();

        // Create a new branch
        create_branch("feature", dir.path()).unwrap();

        let branches = list_branches(dir.path()).unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "feature");
        assert!(!branches[0].is_current);
        assert_eq!(branches[0].commit_hash, commit_hash);
        assert_eq!(branches[1].name, "main");
        assert!(branches[1].is_current);

        // Delete the branch
        let deleted_hash = delete_branch("feature", dir.path()).unwrap();
        assert_eq!(deleted_hash, commit_hash);

        let branches_after = list_branches(dir.path()).unwrap();
        assert_eq!(branches_after.len(), 1);
        assert_eq!(branches_after[0].name, "main");
    }

    #[test]
    fn test_create_branch_already_exists() {
        let (dir, _) = setup_repo_with_commit();
        let result = create_branch("main", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_delete_current_branch_fails() {
        let (dir, _) = setup_repo_with_commit();
        let result = delete_branch("main", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("checked out"));
    }

    #[test]
    fn test_delete_nonexistent_branch_fails() {
        let (dir, _) = setup_repo_with_commit();
        let result = delete_branch("nonexistent", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_create_branch_nested_name() {
        let (dir, commit_hash) = setup_repo_with_commit();
        create_branch("feature/login", dir.path()).unwrap();

        let branches = list_branches(dir.path()).unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "feature/login");
        assert_eq!(branches[0].commit_hash, commit_hash);
    }
}
