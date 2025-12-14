//! Create trees from Index (write-tree)
//!
//! This module handles creating tree objects from the index.
use crate::error::GritError;
use crate::plumbing::objects::{ObjectType, store_object};
use crate::plumbing::index::{Index, IndexEntry};
use std::path::Path;

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

/// Creates a tree object from the Git index.
///
/// This function builds a tree structure reflecting the current state of the index.
/// It recursively constructs tree objects for subdirectories and creates a single
/// root tree object representing the entire staged file structure.
///
/// Used by `grit write-tree` and `grit commit` to create commits from the staging area.
/// The resulting tree hash can be used as the root of a commit.
///
/// # Arguments
///
/// * `index` - The Git index containing the staged files and their metadata
/// * `repo_root` - The root directory of the Git repository
///
/// # Returns
///
/// Returns `Ok(hash)` where `hash` is the 40-character hex SHA-1 hash of the created tree object.
///
/// # Errors
///
/// Returns `GritError` if:
/// - Index entries cannot be processed
/// - Tree objects cannot be stored
/// - Hash decoding fails
///
/// # Algorithm
///
/// 1. Groups index entries by directory level
/// 2. Recursively builds subtrees for directories
/// 3. Creates blob references for files
/// 4. Sorts entries lexicographically (Git standard)
/// 5. Stores the final tree object
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
/// use grit::plumbing::index::read_index;
/// use grit::plumbing::trees::write_tree_from_index;
///
/// let repo_root = Path::new("/path/to/repo");
/// let index = read_index(repo_root).unwrap();
/// let tree_hash = write_tree_from_index(&index, repo_root).unwrap();
/// println!("Tree hash: {}", tree_hash);
/// ```
pub fn write_tree_from_index(index: &Index, repo_root: &Path) -> Result<String, GritError> {
    build_tree_recursive(&index.entries, 0, repo_root)
}

fn build_tree_recursive(entries: &[IndexEntry], prefix_len: usize, repo_root: &Path) -> Result<String, GritError> {
    let mut tree_entries = Vec::new();
    let mut i = 0;

    while i < entries.len() {
        let entry = &entries[i];
        let path = &entry.path;

        // Get the path relative to the current level
        let relative_path = &path[prefix_len..];

        if let Some(slash_pos) = relative_path.find('/') {
            // It's a directory
            let dir_name = &relative_path[..slash_pos];
            let full_dir_prefix = format!("{}{}/", &path[..prefix_len], dir_name);

            // Find all entries in this directory
            let mut j = i + 1;
            while j < entries.len() && entries[j].path.starts_with(&full_dir_prefix) {
                j += 1;
            }

            // Recurse to create subtree
            let subtree_hash_hex = build_tree_recursive(&entries[i..j], full_dir_prefix.len(), repo_root)?;
            let subtree_hash = hex::decode(&subtree_hash_hex)
                .map_err(|_| GritError::CorruptObject("Invalid hash".to_string()))?;

            tree_entries.push(TreeEntry {
                mode: "40000".to_string(),
                name: dir_name.to_string(),
                hash: subtree_hash.try_into().unwrap(),
            });

            i = j;
        } else {
            // It's a file in this directory
            tree_entries.push(TreeEntry {
                mode: format!("{:o}", entry.mode),
                name: relative_path.to_string(),
                hash: entry.hash,
            });
            i += 1;
        }
    }

    // Sort entries by name, treating directories as if they end with '/'
    tree_entries.sort_by(|a, b| {
        let a_name = if a.mode == "40000" { format!("{}/", a.name) } else { a.name.clone() };
        let b_name = if b.mode == "40000" { format!("{}/", b.name) } else { b.name.clone() };
        a_name.cmp(&b_name)
    });

    // Format and store the tree object
    let mut content = Vec::new();
    for entry in tree_entries {
        content.extend_from_slice(format!("{} {}\0", entry.mode, entry.name).as_bytes());
        content.extend_from_slice(&entry.hash);
    }

    store_object(&content, ObjectType::Tree, repo_root)
}

/// Creates a snapshot of a directory structure by recursively traversing it
/// and storing all files and subdirectories as Git objects.
///
/// This implements the Merkle tree concept where the hash of a directory
/// depends on the hashes of all its contents, creating a cryptographically
/// secure snapshot of the entire directory tree.
///
/// The process:
/// 1. Recursively walk the directory (ignoring .grit)
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
/// * `Err(GritError)` - If traversal or storage fails
///
/// # Validation
/// The returned hash should exactly match `git write-tree` for the same directory.
#[cfg(test)]
pub fn create_tree_for_testing(path: &Path, repo_root: &Path) -> Result<String, GritError> {
    let mut index = Index::new();
    add_to_index_recursive(path, repo_root, &mut index)?;
    write_tree_from_index(&index, repo_root)
}

#[cfg(test)]
fn add_to_index_recursive(path: &Path, repo_root: &Path, index: &mut Index) -> Result<(), GritError> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == ".grit" || file_name == "target" || file_name == ".git" {
            continue;
        }

        let file_path = entry.path();
        if file_path.is_dir() {
            add_to_index_recursive(&file_path, repo_root, index)?;
        } else if file_path.is_file() {
            let content = std::fs::read(&file_path)?;
            let hash = store_object(&content, ObjectType::Blob, repo_root)?;
            let hash_bytes = hex::decode(hash)?;
            let mut hash_array = [0u8; 20];
            hash_array.copy_from_slice(&hash_bytes);

            let index_entry = crate::plumbing::index::create_index_entry(&file_path, &hash_array, repo_root)?;
            index.add_entry(index_entry);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plumbing::objects::{ObjectType, read_object};
    use crate::repository::initialize_repo;
    use std::fs;
    use tempfile::TempDir;

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
                (prop::string::string_regex("[a-zA-Z0-9.-]{1,50}").unwrap()
                    .prop_filter("Exclude problematic filenames", |s| s != ".grit" && s != "target" && !s.starts_with('.')),
                 prop::collection::vec(any::<u8>(), 0..1000)),
                1..10
            ).prop_map(|mut files| {
                // Ensure unique filenames by appending index to duplicates
                let mut seen = std::collections::HashSet::new();
                for (i, (filename, _)) in files.iter_mut().enumerate() {
                    if !seen.insert(filename.to_lowercase()) {
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
            let tree_hash = create_tree_for_testing(test_dir.path(), test_dir.path())?;

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
        let tree_hash = create_tree_for_testing(test_dir.path(), test_dir.path()).unwrap();

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
        let tree_hash = create_tree_for_testing(test_dir.path(), test_dir.path()).unwrap();

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
                    let entry_str =
                        String::from_utf8_lossy(&tree_object.content[entry_start..name_end]);
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
        fs::write(
            test_dir.path().join("subdir").join("nested.txt"),
            "Nested content",
        )
        .unwrap();
        fs::write(test_dir.path().join("root.txt"), "Root content").unwrap();

        // Create snapshot
        let tree_hash = create_tree_for_testing(test_dir.path(), test_dir.path()).unwrap();

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
        let tree_hash = create_tree_for_testing(test_dir.path(), test_dir.path()).unwrap();

        // Read back the tree object
        let tree_object = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_object.obj_type, ObjectType::Tree);

        // Should be empty (only contains .grit which is ignored)
        assert!(tree_object.content.is_empty());
    }
}
