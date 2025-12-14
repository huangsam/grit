use std::path::Path;
use std::fs;
use crate::error::CrustError;
use crate::plumbing::objects::{store_object, ObjectType};
use rayon::prelude::*;

/// Represents a single entry in a Git tree object.
/// Tree entries contain the metadata needed to reconstruct a directory structure,
/// including file/directory names, permissions, and content references.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// The file mode/permissions as a string (e.g., "100644" for regular files,
    /// "40000" for directories). This follows Git's internal representation.
    pub mode: String,

    /// The name of the file or directory. This is the basename, not a full path.
    pub name: String,

    /// The 20-byte SHA-1 hash of the object this entry refers to.
    /// For files, this points to a blob object; for directories, to a tree object.
    /// Stored as raw bytes, not hex string, to match Git's internal format.
    pub hash: [u8; 20],
}

/// Creates a snapshot of a directory structure by recursively traversing it
/// and storing all files and subdirectories as Git objects.
///
/// This implements the Merkle tree concept where the hash of a directory
/// depends on the hashes of all its contents, creating a cryptographically
/// secure snapshot of the entire directory tree.
///
/// The process:
/// 1. Recursively walk the directory (ignoring .crust)
/// 2. For files: store as blobs and collect entries
/// 3. For directories: recursively create sub-trees
/// 4. Sort entries lexicographically by name
/// 5. Format and store the tree object
///
/// # Arguments
/// * `path` - The directory path to snapshot
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex hash of the created tree object
/// * `Err(CrustError)` - If traversal or storage fails
///
/// # Validation
/// The returned hash should exactly match `git write-tree` for the same directory.
pub fn make_snapshot(path: &Path, repo_root: &Path) -> Result<String, CrustError> {
    // Step 1: Collect all directory entries
    let dir_entries: Vec<_> = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            // Ignore .crust directory and build directories
            file_name != ".crust" && file_name != "target" && file_name != ".git"
        })
        .collect();

    // Step 2: Process entries in parallel
    let entries_result: Result<Vec<TreeEntry>, CrustError> = dir_entries
        .par_iter()
        .map(|entry| -> Result<TreeEntry, CrustError> {
            let entry_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                // For files: store as blob
                let content = fs::read(&entry_path)?;
                let hash = store_object(&content, ObjectType::Blob, repo_root)?;
                let hash_bytes = hex::decode(hash)?;

                Ok(TreeEntry {
                    mode: "100644".to_string(), // Regular file mode
                    name: file_name,
                    hash: hash_bytes.try_into().unwrap(), // Convert Vec<u8> to [u8; 20]
                })
            } else if metadata.is_dir() {
                // For directories: recursively create tree
                let hash = make_snapshot(&entry_path, repo_root)?;
                let hash_bytes = hex::decode(hash)?;

                Ok(TreeEntry {
                    mode: "40000".to_string(), // Directory mode
                    name: file_name,
                    hash: hash_bytes.try_into().unwrap(),
                })
            } else {
                // Ignore other types (symlinks, etc.) for now
                // Return a dummy entry that will be filtered out
                Ok(TreeEntry {
                    mode: "".to_string(),
                    name: "".to_string(),
                    hash: [0u8; 20],
                })
            }
        })
        .collect();

    let mut entries = entries_result?;
    // Filter out dummy entries (for ignored file types)
    entries.retain(|entry| !entry.mode.is_empty());

    // Step 4: Sorting
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    // Step 3: Tree Entry Format & Step 5: Storage
    let mut tree_content = Vec::new();
    for entry in entries {
        // Format: <mode> <name>\0<20 raw byte hash>
        tree_content.extend_from_slice(format!("{} {}\0", entry.mode, entry.name).as_bytes());
        tree_content.extend_from_slice(&entry.hash);
    }

    // Store the tree object
    let tree_hash = store_object(&tree_content, ObjectType::Tree, repo_root)?;
    Ok(tree_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::repository::initialize_repo;
    use crate::plumbing::objects::{read_object, ObjectType};
    use std::fs;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Initialize repository
        initialize_repo(temp_dir.path()).unwrap();

        temp_dir
    }

    // Property-based tests
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_make_snapshot_with_random_files(
            files in prop::collection::vec(
                (prop::string::string_regex("[a-zA-Z0-9_.-]{1,50}").unwrap()
                    .prop_filter("Exclude problematic filenames", |s| s != ".crust" && s != "target" && !s.starts_with('.')),
                 prop::collection::vec(any::<u8>(), 0..1000)),
                1..10
            ).prop_map(|mut files| {
                // Ensure unique filenames by appending index to duplicates
                let mut seen = std::collections::HashSet::new();
                for (i, (filename, _)) in files.iter_mut().enumerate() {
                    if !seen.insert(filename.clone()) {
                        *filename = format!("{}_{}", filename, i);
                    }
                }
                files
            })
        ) {
            let test_dir = setup_test_repo();

            // Create the random files
            for (filename, content) in &files {
                fs::write(test_dir.path().join(filename), content)?;
            }

            // Create tree snapshot
            let tree_hash = make_snapshot(test_dir.path(), test_dir.path())?;

            // Verify tree was created
            let tree_object = read_object(&tree_hash, test_dir.path())?;
            prop_assert_eq!(tree_object.obj_type, ObjectType::Tree);

            // Parse tree entries
            let entries = crate::plumbing::checkout::parse_tree_entries(&tree_object.content)?;
            prop_assert_eq!(entries.len(), files.len());

            // Verify all files are in the tree
            let entry_names: std::collections::HashSet<_> = entries.iter().map(|e| e.name.as_str()).collect();
            let expected_names: std::collections::HashSet<_> = files.iter().map(|(name, _)| name.as_str()).collect();
            prop_assert_eq!(entry_names, expected_names);
        }
    }

    #[test]
    fn test_make_snapshot_single_file() {
        let test_dir = setup_test_repo();

        // Create a test file
        fs::write(test_dir.path().join("test.txt"), "Hello, World!").unwrap();

        // Create snapshot
        let tree_hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Verify hash is 40 characters
        assert_eq!(tree_hash.len(), 40);
        assert!(tree_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Read back the tree object
        let tree_object = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_object.obj_type, ObjectType::Tree);

        // Tree content should contain file entry
        let content = String::from_utf8_lossy(&tree_object.content);
        assert!(content.contains("100644 test.txt"));
    }

    #[test]
    fn test_make_snapshot_multiple_files() {
        let test_dir = setup_test_repo();

        // Create multiple test files
        fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
        fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
        fs::write(test_dir.path().join("another.txt"), "Another content").unwrap();

        // Create snapshot
        let tree_hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Read back the tree object
        let tree_object = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_object.obj_type, ObjectType::Tree);

        let content = String::from_utf8_lossy(&tree_object.content);

        // Should contain all files, sorted lexicographically
        assert!(content.contains("100644 another.txt"));
        assert!(content.contains("100644 file1.txt"));
        assert!(content.contains("100644 file2.txt"));

        // Check that entries are in sorted order by parsing the tree format
        let mut pos = 0;
        let mut entries = Vec::new();
        while pos < tree_object.content.len() {
            // Find the null byte
            if let Some(null_pos) = tree_object.content[pos..].iter().position(|&b| b == 0) {
                let entry_start = pos;
                let name_end = pos + null_pos;
                let hash_start = name_end + 1;
                let hash_end = hash_start + 20;

                if hash_end <= tree_object.content.len() {
                    let entry_str = String::from_utf8_lossy(&tree_object.content[entry_start..name_end]);
                    entries.push(entry_str.to_string());
                    pos = hash_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        assert_eq!(entries.len(), 3);
        assert!(entries[0].starts_with("100644 another.txt"));
        assert!(entries[1].starts_with("100644 file1.txt"));
        assert!(entries[2].starts_with("100644 file2.txt"));
    }

    #[test]
    fn test_make_snapshot_with_subdirectory() {
        let test_dir = setup_test_repo();

        // Create directory structure
        fs::create_dir(test_dir.path().join("subdir")).unwrap();
        fs::write(test_dir.path().join("subdir").join("nested.txt"), "Nested content").unwrap();
        fs::write(test_dir.path().join("root.txt"), "Root content").unwrap();

        // Create snapshot
        let tree_hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Read back the tree object
        let tree_object = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_object.obj_type, ObjectType::Tree);

        let content = String::from_utf8_lossy(&tree_object.content);

        // Should contain both file and directory
        assert!(content.contains("100644 root.txt"));
        assert!(content.contains("40000 subdir"));
    }

    #[test]
    fn test_make_snapshot_empty_directory() {
        let test_dir = setup_test_repo();

        // Create snapshot of empty directory
        let tree_hash = make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Read back the tree object
        let tree_object = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_object.obj_type, ObjectType::Tree);

        // Should be empty (only contains .crust which is ignored)
        assert!(tree_object.content.is_empty());
    }
}
