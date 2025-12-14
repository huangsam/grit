//! Implementation of the `grit reset` porcelain command
//!
//! This module handles resetting the current HEAD to a specified state.

use crate::error::GritError;
use crate::plumbing::commits::update_ref;
use crate::plumbing::index::{Index, IndexEntry, write_index};
use crate::plumbing::objects::{read_object, ObjectType};
use crate::plumbing::checkout::{restore_snapshot, parse_tree_entries};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Copy)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

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
