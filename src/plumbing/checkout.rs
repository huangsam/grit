use crate::error::CrustError;
use crate::plumbing::objects::{read_object, ObjectType};
use crate::cache;
use sha1::{Digest, Sha1};
use hex;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Restores a tree or commit snapshot to the working directory.
///
/// This function recursively reads tree objects and writes the corresponding
/// blob contents to files in the working directory. It handles both direct
/// tree hashes and commit hashes (by extracting the tree reference).
///
/// # Arguments
/// * `hash` - The hash of the tree or commit to restore
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(())` - If the snapshot was successfully restored
/// * `Err(CrustError)` - If reading objects or writing files fails
///
/// # Errors
/// * `ObjectNotFound` - If the specified hash doesn't exist
/// * `CorruptObject` - If the object data is malformed
/// * `Io` - If file system operations fail
pub fn restore_snapshot(hash: &str, repo_root: &Path) -> Result<(), CrustError> {
    // Read the object
    let object = read_object(hash, repo_root)?;

    let tree_hash = match object.obj_type {
        ObjectType::Commit => {
            // Extract tree hash from commit
            let content = String::from_utf8_lossy(&object.content);
            extract_tree_hash_from_commit(&content)?
        }
        ObjectType::Tree => hash.to_string(),
        ObjectType::Blob => {
            return Err(CrustError::RepositoryError(
                "Cannot checkout a blob object".to_string()
            ));
        }
    };

    // Restore the tree
    restore_tree(&tree_hash, repo_root, Path::new(""))
}

/// Extracts the tree hash from a commit object's content
fn extract_tree_hash_from_commit(commit_content: &str) -> Result<String, CrustError> {
    for line in commit_content.lines() {
        if let Some(tree_hash) = line.strip_prefix("tree ") {
            return Ok(tree_hash.to_string());
        }
    }
    Err(CrustError::CorruptObject("Commit missing tree reference".to_string()))
}

/// Recursively restores a tree object to the working directory
fn restore_tree(tree_hash: &str, repo_root: &Path, current_path: &Path) -> Result<(), CrustError> {
    let tree_object = read_object(tree_hash, repo_root)?;

    if tree_object.obj_type != ObjectType::Tree {
        return Err(CrustError::CorruptObject("Expected tree object".to_string()));
    }

    // Parse tree entries
    let entries = parse_tree_entries(&tree_object.content)?;

    for entry in entries {
        let entry_path = repo_root.join(current_path).join(&entry.name);

        if entry.mode == "40000" {
            // Directory (tree)
            fs::create_dir_all(&entry_path)?;
            let sub_tree_hash = hex::encode(entry.hash);
            restore_tree(&sub_tree_hash, repo_root, &current_path.join(&entry.name))?;
        } else {
            // File (blob)
            let blob_object = read_object(&hex::encode(entry.hash), repo_root)?;

            if blob_object.obj_type != ObjectType::Blob {
                return Err(CrustError::CorruptObject("Tree entry points to non-blob".to_string()));
            }

            // Write file content
            fs::write(&entry_path, &blob_object.content)?;

            // Set file permissions (basic implementation)
            if let Ok(metadata) = fs::metadata(&entry_path) {
                let mut permissions = metadata.permissions();
                // For now, set executable bit if mode indicates executable
                if entry.mode == "100755" {
                    permissions.set_mode(0o755);
                } else {
                    permissions.set_mode(0o644);
                }
                fs::set_permissions(&entry_path, permissions)?;
            }
        }
    }

    Ok(())
}

/// Represents a tree entry parsed from tree object content
#[derive(Debug)]
#[derive(Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub hash: [u8; 20],
}

/// Parses tree entries from raw tree content
pub fn parse_tree_entries(content: &[u8]) -> Result<Vec<TreeEntry>, CrustError> {
    // Compute content hash for caching
    let content_hash = hex::encode(Sha1::digest(content));

    // Check cache first
    if let Some(cached_entries) = cache::GLOBAL_CACHE.tree_cache.get(&content_hash) {
        return Ok(cached_entries);
    }

    let mut entries = Vec::new();
    let mut pos = 0;

    while pos < content.len() {
        // Find null byte separating mode/name from hash
        let null_pos = content[pos..].iter().position(|&b| b == 0)
            .ok_or_else(|| CrustError::CorruptObject("Invalid tree entry format".to_string()))?;

        let header = &content[pos..pos + null_pos];
        pos += null_pos + 1;

        // Parse mode and name
        let header_str = String::from_utf8_lossy(header);
        let space_pos = header_str.find(' ')
            .ok_or_else(|| CrustError::CorruptObject("Invalid tree entry header".to_string()))?;

        let mode = header_str[..space_pos].to_string();
        let name = header_str[space_pos + 1..].to_string();

        // Read 20-byte hash
        if pos + 20 > content.len() {
            return Err(CrustError::CorruptObject("Tree entry truncated".to_string()));
        }

        let mut hash = [0u8; 20];
        hash.copy_from_slice(&content[pos..pos + 20]);
        pos += 20;

        entries.push(TreeEntry { mode, name, hash });
    }

    // Cache the parsed entries
    cache::GLOBAL_CACHE.tree_cache.put(content_hash, entries.clone());

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::repository::initialize_repo;
    use crate::plumbing::trees::make_snapshot;
    use std::fs;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        initialize_repo(temp_dir.path()).unwrap();
        temp_dir
    }

    #[test]
    fn test_restore_snapshot_tree() {
        let test_dir = setup_test_repo();

        // Create some test files
        fs::write(test_dir.path().join("hello.txt"), "Hello World").unwrap();
        fs::create_dir(test_dir.path().join("subdir")).unwrap();
        fs::write(test_dir.path().join("subdir").join("nested.txt"), "Nested content").unwrap();

        // Create tree snapshot
        let tree_hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Clear working directory
        fs::remove_file(test_dir.path().join("hello.txt")).unwrap();
        fs::remove_dir_all(test_dir.path().join("subdir")).unwrap();

        // Restore snapshot
        restore_snapshot(&tree_hash, test_dir.path()).unwrap();

        // Verify files were restored
        assert!(test_dir.path().join("hello.txt").exists());
        assert_eq!(fs::read_to_string(test_dir.path().join("hello.txt")).unwrap(), "Hello World");
        assert!(test_dir.path().join("subdir").join("nested.txt").exists());
        assert_eq!(fs::read_to_string(test_dir.path().join("subdir").join("nested.txt")).unwrap(), "Nested content");
    }

    // Property-based tests
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_parse_tree_entries_with_random_data(
            entries in prop::collection::vec(
                (prop::string::string_regex(r"100644|100755|40000").unwrap(), // Valid modes
                 prop::string::string_regex("[a-zA-Z0-9_.-]{1,50}").unwrap()
                    .prop_filter("Exclude problematic filenames", |s| s != ".crust" && s != "target" && !s.starts_with('.')),
                 prop::array::uniform20(any::<u8>())), // 20-byte hash
                1..5
            )
        ) {
            // Build tree content from random entries
            let mut content = Vec::new();
            for (mode, name, hash) in &entries {
                content.extend_from_slice(format!("{} {}\0", mode, name).as_bytes());
                content.extend_from_slice(hash);
            }

            // Parse the tree entries
            let parsed_entries = parse_tree_entries(&content)?;
            prop_assert_eq!(parsed_entries.len(), entries.len());

            // Verify each entry matches
            for (i, (expected_mode, expected_name, expected_hash)) in entries.iter().enumerate() {
                let actual = &parsed_entries[i];
                prop_assert_eq!(&actual.mode, expected_mode);
                prop_assert_eq!(&actual.name, expected_name);
                prop_assert_eq!(&actual.hash, expected_hash);
            }
        }
    }
}
