//! Implementation of the `grit status` porcelain command
//!
//! This module shows the status of the working directory and staging area,
//! including staged changes, unstaged changes, and untracked files.

use crate::error::GritError;
use crate::plumbing::checkout::parse_tree_entries;
use crate::plumbing::commits::get_current_commit;
use crate::plumbing::index::read_index;
use crate::plumbing::objects::read_object;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Show the status of the working directory and staging area
pub fn show_status(repo_root: &Path) -> Result<(), GritError> {
    // Read the current index
    let index = read_index(repo_root)?;

    // Get the HEAD commit tree entries
    let head_tree_entries = get_head_tree_entries(repo_root)?;

    // Create a map from path to hash for HEAD
    let mut head_hashes = HashMap::new();
    for entry in head_tree_entries {
        head_hashes.insert(entry.name, entry.hash);
    }

    // Collect all files in the working directory with their hashes
    let mut working_files = HashMap::new();
    collect_working_files(repo_root, &mut working_files, repo_root)?;

    // Categorize changes
    let mut staged_changes = Vec::new();
    let mut unstaged_changes = Vec::new();
    let mut untracked_files = Vec::new();

    // Check staged changes: compare index with HEAD
    for entry in &index.entries {
        let path = &entry.path;
        let head_hash = head_hashes.get(path).cloned();

        if head_hash != Some(entry.hash) {
            let change_type = if head_hash.is_none() {
                "new file:"
            } else {
                "modified:"
            };
            staged_changes.push(format!("{} {}", change_type, path));
        }
    }

    // Check unstaged changes: compare working directory with index
    for entry in &index.entries {
        let path = &entry.path;

        if let Some(working_hash) = working_files.get(path) {
            if *working_hash != entry.hash {
                unstaged_changes.push(format!("modified: {}", path));
            }
        } else {
            // File exists in index but not in working directory
            unstaged_changes.push(format!("deleted: {}", path));
        }
    }

    // Find untracked files: files in working directory but not in index
    for (path, _) in &working_files {
        if !index.entries.iter().any(|e| &e.path == path) {
            untracked_files.push(path.clone());
        }
    }

    // Display results
    if !staged_changes.is_empty() {
        println!("Changes to be committed:");
        for change in &staged_changes {
            println!("  {}", change);
        }
        println!();
    }

    if !unstaged_changes.is_empty() {
        println!("Changes not staged for commit:");
        for change in &unstaged_changes {
            println!("  {}", change);
        }
        println!();
    }

    if !untracked_files.is_empty() {
        println!("Untracked files:");
        for file in &untracked_files {
            println!("  {}", file);
        }
        println!();
    }

    if staged_changes.is_empty() && unstaged_changes.is_empty() && untracked_files.is_empty() {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
}

/// Get the tree entries from the HEAD commit
fn get_head_tree_entries(
    repo_root: &Path,
) -> Result<Vec<crate::plumbing::checkout::TreeEntry>, GritError> {
    let head_commit = get_current_commit(repo_root)?;

    if let Some(commit_hash) = head_commit {
        // Read the commit object
        let commit_obj = read_object(&commit_hash, repo_root)?;
        let commit_content = String::from_utf8_lossy(&commit_obj.content);

        // Extract the tree hash
        let tree_line = commit_content
            .lines()
            .find(|line| line.starts_with("tree "))
            .ok_or_else(|| GritError::RepositoryError("Invalid commit format".to_string()))?;
        let tree_hash = &tree_line[5..];

        // Read the tree object
        let tree_obj = read_object(tree_hash, repo_root)?;

        // Parse the tree entries
        parse_tree_entries(&tree_obj.content)
    } else {
        // No commits yet
        Ok(Vec::new())
    }
}

/// Recursively collect all files in the working directory with their SHA-1 hashes
fn collect_working_files(
    dir: &Path,
    files: &mut HashMap<String, [u8; 20]>,
    repo_root: &Path,
) -> Result<(), GritError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip the .grit directory
        if path.file_name().is_some_and(|n| n == ".grit") {
            continue;
        }

        if path.is_dir() {
            collect_working_files(&path, files, repo_root)?;
        } else if path.is_file() {
            // Read file content and compute hash
            let content = fs::read(&path)?;

            // Compute Git object hash (blob)
            let header = format!("blob {}\0", content.len());
            let mut hasher = Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(&content);
            let hash = hasher.finalize();

            let mut hash_array = [0u8; 20];
            hash_array.copy_from_slice(&hash);

            // Get relative path from repository root
            let relative_path = path
                .strip_prefix(repo_root)
                .map_err(|_| GritError::RepositoryError("File outside repository".to_string()))?
                .to_string_lossy()
                .to_string();

            files.insert(relative_path, hash_array);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Initialize repository
        crate::repository::initialize_repo(repo_root).unwrap();

        temp_dir
    }

    #[test]
    fn test_status_clean_repo() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Status should show clean
        show_status(repo_root).unwrap();
    }

    #[test]
    fn test_status_with_untracked_file() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create an untracked file
        fs::write(repo_root.join("untracked.txt"), b"content").unwrap();

        // Status should show untracked file
        show_status(repo_root).unwrap();
    }
}
