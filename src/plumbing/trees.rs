use std::path::Path;
use std::fs;
use crate::error::CrustError;
use crate::plumbing::objects::{store_object, ObjectType};

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
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex hash of the created tree object
/// * `Err(CrustError)` - If traversal or storage fails
///
/// # Validation
/// The returned hash should exactly match `git write-tree` for the same directory.
pub fn make_snapshot(path: &Path) -> Result<String, CrustError> {
    let mut entries = Vec::new();

    // Step 1: Directory Traversal
    let dir_entries = fs::read_dir(path)?;

    for entry in dir_entries {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Ignore .crust directory
        if file_name == ".crust" {
            continue;
        }

        let metadata = entry.metadata()?;

        // Step 2: Entry Handling
        if metadata.is_file() {
            // For files: store as blob
            let content = fs::read(&entry_path)?;
            let hash = store_object(&content, ObjectType::Blob)?;
            let hash_bytes = hex::decode(hash)?;

            entries.push(TreeEntry {
                mode: "100644".to_string(), // Regular file mode
                name: file_name,
                hash: hash_bytes.try_into().unwrap(), // Convert Vec<u8> to [u8; 20]
            });
        } else if metadata.is_dir() {
            // For directories: recursively create tree
            let hash = make_snapshot(&entry_path)?;
            let hash_bytes = hex::decode(hash)?;

            entries.push(TreeEntry {
                mode: "40000".to_string(), // Directory mode
                name: file_name,
                hash: hash_bytes.try_into().unwrap(),
            });
        }
        // Ignore other types (symlinks, etc.) for now
    }

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
    let tree_hash = store_object(&tree_content, ObjectType::Tree)?;
    Ok(tree_hash)
}
