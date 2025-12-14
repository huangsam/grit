use std::collections::HashMap;
use std::path::{Path, PathBuf};
use hex;

use crate::error::GritError;
use crate::repository::Repository;
use crate::plumbing::objects;
use crate::plumbing::checkout::TreeEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
    TypeChange,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub path: PathBuf,
    pub mode_a: u32,
    pub hash_a: String,
    pub mode_b: u32,
    pub hash_b: String,
    pub status: DiffStatus,
}

pub fn compare_trees(
    repo: &Repository,
    tree_hash_a: &str,
    tree_hash_b: &str,
    prefix: &Path,
) -> Result<Vec<DiffEntry>, GritError> {
    let tree_a = objects::read_tree(repo, tree_hash_a)?;
    let tree_b = objects::read_tree(repo, tree_hash_b)?;

    let entries_a: HashMap<&str, &TreeEntry> = tree_a.entries.iter().map(|e| (e.name.as_str(), e)).collect();
    let entries_b: HashMap<&str, &TreeEntry> = tree_b.entries.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut diffs = Vec::new();

    // Handle entries only in A (deletions)
    for (name, entry_a) in &entries_a {
        if !entries_b.contains_key(name) {
            let full_path = prefix.join(name);
            let mode_a = u32::from_str_radix(&entry_a.mode, 8).unwrap_or(0);
            diffs.push(DiffEntry {
                path: full_path,
                mode_a,
                hash_a: hex::encode(entry_a.hash),
                mode_b: 0,
                hash_b: String::new(),
                status: DiffStatus::Deleted,
            });
        }
    }

    // Handle entries only in B (additions) and in both
    for (name, entry_b) in &entries_b {
        let full_path = prefix.join(name);
        if let Some(entry_a) = entries_a.get(name) {
            // In both
            if entry_a.hash != entry_b.hash {
                let mode_a = u32::from_str_radix(&entry_a.mode, 8).unwrap_or(0);
                let mode_b = u32::from_str_radix(&entry_b.mode, 8).unwrap_or(0);
                diffs.push(DiffEntry {
                    path: full_path,
                    mode_a,
                    hash_a: hex::encode(entry_a.hash),
                    mode_b,
                    hash_b: hex::encode(entry_b.hash),
                    status: if mode_a != mode_b { DiffStatus::TypeChange } else { DiffStatus::Modified },
                });
            }
            // If same, do nothing
        } else {
            // Only in B
            let full_path = prefix.join(name);
            let mode_b = u32::from_str_radix(&entry_b.mode, 8).unwrap_or(0);
            diffs.push(DiffEntry {
                path: full_path,
                mode_a: 0,
                hash_a: String::new(),
                mode_b,
                hash_b: hex::encode(entry_b.hash),
                status: DiffStatus::Added,
            });
        }
    }

    // Recurse into subtrees
    for (name, entry_a) in &entries_a {
        if let Some(entry_b) = entries_b.get(name) {
            let mode_a = u32::from_str_radix(&entry_a.mode, 8).unwrap_or(0);
            let mode_b = u32::from_str_radix(&entry_b.mode, 8).unwrap_or(0);
            if mode_a == 0o040000 && mode_b == 0o040000 && entry_a.hash != entry_b.hash {
                let sub_diffs = compare_trees(repo, &hex::encode(entry_a.hash), &hex::encode(entry_b.hash), &prefix.join(name))?;
                diffs.extend(sub_diffs);
            }
        }
    }

    Ok(diffs)
}

pub fn get_file_deltas(content_a: &str, content_b: &str, path: &Path) -> (String, usize, usize) {
    let lines_a: Vec<&str> = content_a.lines().collect();
    let lines_b: Vec<&str> = content_b.lines().collect();

    let mut output = format!("--- a/{}\n+++ b/{}\n", path.display(), path.display());
    let mut insertions = 0;
    let mut deletions = 0;

    let mut i = 0;
    let mut j = 0;

    while i < lines_a.len() && j < lines_b.len() {
        if lines_a[i] == lines_b[j] {
            i += 1;
            j += 1;
        } else {
            // Find the end of the difference
            let start_i = i;
            let start_j = j;
            while i < lines_a.len() && j < lines_b.len() && lines_a[i] != lines_b[j] {
                i += 1;
                j += 1;
            }
            let del_count = i - start_i;
            let add_count = j - start_j;
            deletions += del_count;
            insertions += add_count;
            output.push_str(&format!("@@ -{},{} +{},{} @@\n", start_i + 1, del_count, start_j + 1, add_count));
            for line in lines_a.iter().take(i).skip(start_i) {
                output.push_str(&format!("-{}\n", line));
            }
            for line in lines_b.iter().take(j).skip(start_j) {
                output.push_str(&format!("+{}\n", line));
            }
        }
    }

    // Handle remaining lines
    if i < lines_a.len() {
        let del_count = lines_a.len() - i;
        deletions += del_count;
        output.push_str(&format!("@@ -{},{} +{},{} @@\n", i + 1, del_count, j + 1, 0));
        for line in lines_a.iter().skip(i) {
            output.push_str(&format!("-{}\n", line));
        }
    }
    if j < lines_b.len() {
        let add_count = lines_b.len() - j;
        insertions += add_count;
        output.push_str(&format!("@@ -{},{} +{},{} @@\n", i + 1, 0, j + 1, add_count));
        for line in lines_b.iter().skip(j) {
            output.push_str(&format!("+{}\n", line));
        }
    }

    (output, insertions, deletions)
}
