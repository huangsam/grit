use std::path::Path;
use crate::error::CrustError;

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
/// 1. Recursively walk the directory (ignoring .git)
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
    // TODO: Implement
    Ok(String::new())
}
