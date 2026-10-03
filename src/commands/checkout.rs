//! # Git Checkout Command Implementation
//!
//! This module implements the `grit checkout` porcelain command, which switches
//! branches or restores working tree files from commits or snapshots.
//!
//! ## Overview
//!
//! Checkout allows users to:
//! - Switch to an existing local branch: `grit checkout <branch>`
//! - Create and switch to a new branch: `grit checkout -b <new_branch>`
//! - Check out a specific commit (detached HEAD): `grit checkout <commit_hash>`
//! - Restore snapshots from tree objects: `grit checkout <tree_hash>`

use crate::error::GritError;
use crate::plumbing::checkout::restore_snapshot;
use crate::plumbing::commits::{get_current_commit, validate_ref_name};
use crate::plumbing::index::{read_index, write_index};
use crate::plumbing::objects::{ObjectType, read_object};
use crate::plumbing::trees::build_index_from_tree;
use std::fs;
use std::path::Path;

/// Extracts the tree hash from a commit object's content.
fn extract_tree_hash(commit_content: &str) -> Result<String, GritError> {
    commit_content
        .lines()
        .find(|line| line.starts_with("tree "))
        .map(|line| line[5..].to_string())
        .ok_or_else(|| GritError::CorruptObject("Commit missing tree hash".to_string()))
}

/// Switched working directory and index to match a given commit.
fn switch_to_commit(commit_hash: &str, repo_root: &Path) -> Result<(), GritError> {
    let object = read_object(commit_hash, repo_root)?;
    if object.obj_type != ObjectType::Commit {
        return Err(GritError::repo(format!(
            "Object '{}' is not a commit",
            commit_hash
        )));
    }

    let old_index = read_index(repo_root).ok();

    let commit_content = String::from_utf8_lossy(&object.content);
    let tree_hash = extract_tree_hash(&commit_content)?;

    let new_index = build_index_from_tree(&tree_hash, repo_root)?;
    write_index(&new_index, repo_root)?;

    // Delete files present in old index but absent in new index
    if let Some(old) = old_index {
        for entry in &old.entries {
            if !new_index.entries.iter().any(|e| e.path == entry.path) {
                let path = repo_root.join(&entry.path);
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    restore_snapshot(commit_hash, repo_root)?;
    Ok(())
}

/// Executes the checkout command.
pub fn checkout(target: &str, new_branch: bool, repo_root: &Path) -> Result<(), GritError> {
    let head_path = repo_root.join(".grit").join("HEAD");

    if new_branch {
        validate_ref_name(target)?;

        let branch_path = repo_root
            .join(".grit")
            .join("refs")
            .join("heads")
            .join(target);

        if branch_path.exists() {
            return Err(GritError::repo(format!(
                "A branch named '{}' already exists",
                target
            )));
        }

        let current_commit = get_current_commit(repo_root)?.ok_or_else(|| {
            GritError::repo("Cannot checkout -b: not a valid object name: 'HEAD'")
        })?;

        if let Some(parent) = branch_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&branch_path, format!("{}\n", current_commit))?;
        fs::write(&head_path, format!("ref: refs/heads/{}\n", target))?;
        println!("Switched to a new branch '{}'", target);
        return Ok(());
    }

    // Check if target matches a local branch
    let branch_path = repo_root
        .join(".grit")
        .join("refs")
        .join("heads")
        .join(target);

    if branch_path.exists() {
        let commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
        switch_to_commit(&commit_hash, repo_root)?;
        fs::write(&head_path, format!("ref: refs/heads/{}\n", target))?;
        println!("Switched to branch '{}'", target);
        return Ok(());
    }

    // Try treating target as an object hash (commit or tree)
    match read_object(target, repo_root) {
        Ok(object) => match object.obj_type {
            ObjectType::Commit => {
                switch_to_commit(target, repo_root)?;
                fs::write(&head_path, format!("{}\n", target))?;
                let short_hash = if target.len() >= 7 {
                    &target[..7]
                } else {
                    target
                };
                println!(
                    "Note: switching to '{}'.\nHEAD is now at {}",
                    target, short_hash
                );
                Ok(())
            }
            ObjectType::Tree => {
                restore_snapshot(target, repo_root)?;
                let short_hash = if target.len() >= 8 {
                    &target[..8]
                } else {
                    target
                };
                println!("Restored snapshot {}", short_hash);
                Ok(())
            }
            ObjectType::Blob => Err(GritError::repo("Cannot checkout a blob object")),
        },
        Err(_) => Err(GritError::repo(format!(
            "error: pathspec '{}' did not match any file(s) known to grit",
            target
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::add::add_files;
    use crate::plumbing::commits::{create_commit, update_ref};
    use crate::repository::initialize_repo;
    use tempfile::TempDir;

    fn setup_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        initialize_repo(dir.path()).unwrap();
        dir
    }

    #[test]
    fn test_checkout_new_branch() {
        let dir = setup_repo();
        fs::write(dir.path().join("file.txt"), "v1").unwrap();
        add_files(&["file.txt".to_string()], dir.path()).unwrap();

        let index = read_index(dir.path()).unwrap();
        let tree_hash = crate::plumbing::trees::write_tree_from_index(&index, dir.path()).unwrap();
        let commit_hash = create_commit(&tree_hash, None, "commit 1", dir.path()).unwrap();
        update_ref("refs/heads/main", &commit_hash, dir.path()).unwrap();

        // Checkout -b feature
        checkout("feature", true, dir.path()).unwrap();

        let head = fs::read_to_string(dir.path().join(".grit/HEAD")).unwrap();
        assert_eq!(head.trim(), "ref: refs/heads/feature");

        let feature_ref = fs::read_to_string(dir.path().join(".grit/refs/heads/feature")).unwrap();
        assert_eq!(feature_ref.trim(), commit_hash);
    }

    #[test]
    fn test_checkout_existing_branch() {
        let dir = setup_repo();
        fs::write(dir.path().join("file.txt"), "v1").unwrap();
        add_files(&["file.txt".to_string()], dir.path()).unwrap();

        let index = read_index(dir.path()).unwrap();
        let tree_hash = crate::plumbing::trees::write_tree_from_index(&index, dir.path()).unwrap();
        let commit_hash = create_commit(&tree_hash, None, "commit 1", dir.path()).unwrap();
        update_ref("refs/heads/main", &commit_hash, dir.path()).unwrap();

        // Create feature branch
        crate::commands::branch::create_branch("feature", dir.path()).unwrap();

        // Switch to feature branch
        checkout("feature", false, dir.path()).unwrap();
        let head = fs::read_to_string(dir.path().join(".grit/HEAD")).unwrap();
        assert_eq!(head.trim(), "ref: refs/heads/feature");

        // Switch back to main
        checkout("main", false, dir.path()).unwrap();
        let head = fs::read_to_string(dir.path().join(".grit/HEAD")).unwrap();
        assert_eq!(head.trim(), "ref: refs/heads/main");
    }

    #[test]
    fn test_checkout_nonexistent_branch_fails() {
        let dir = setup_repo();
        let res = checkout("ghost", false, dir.path());
        assert!(res.is_err());
    }
}
