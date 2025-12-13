use crate::error::CrustError;
use std::path::Path;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::plumbing::objects::{store_object, ObjectType};

/// Creates a new commit object linking a tree snapshot to the commit history.
/// Commits form the backbone of Git's DAG (Directed Acyclic Graph) by referencing
/// a tree object (the file structure) and optionally a parent commit.
///
/// The commit object contains:
/// - Reference to the tree object representing the file snapshot
/// - Optional reference to parent commit(s) for history
/// - Author and committer information with timestamps
/// - Commit message describing the changes
///
/// # Arguments
/// * `tree_hash` - The 40-character hex hash of the tree object for this commit
/// * `parent_hash` - Optional hash of the parent commit (None for initial commit)
/// * `message` - The commit message describing the changes
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex hash of the created commit object
/// * `Err(CrustError)` - If commit creation or storage fails
pub fn create_commit(tree_hash: &str, parent_hash: Option<&str>, message: &str) -> Result<String, CrustError> {
    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CrustError::RepositoryError("System time before Unix epoch".to_string()))?;
    let timestamp = now.as_secs();
    let timezone = "+0000"; // UTC

    // Default author/committer info (in a real implementation, this would come from config)
    let author = format!("Crust User <crust@example.com> {} {}", timestamp, timezone);
    let committer = author.clone();

    // Build commit content
    let mut commit_content = format!("tree {}\n", tree_hash);

    if let Some(parent) = parent_hash {
        commit_content.push_str(&format!("parent {}\n", parent));
    }

    commit_content.push_str(&format!("author {}\n", author));
    commit_content.push_str(&format!("committer {}\n", committer));
    commit_content.push_str(&format!("\n{}\n", message));

    // Store the commit object
    let hash = store_object(commit_content.as_bytes(), ObjectType::Commit)?;

    Ok(hash)
}

/// Updates a Git reference (like HEAD or a branch) to point to a new commit.
/// References are essentially pointers to commits that track the current state
/// of branches, HEAD, and other important commit references.
///
/// This function writes the commit hash to the specified reference file,
/// updating what the reference points to. For example, updating HEAD to
/// point to a new commit after a successful commit operation.
///
/// # Arguments
/// * `ref_name` - The name of the reference to update (e.g., "HEAD", "refs/heads/main")
/// * `hash` - The 40-character hex hash of the commit to point the reference to
///
/// # Returns
/// * `Ok(())` - If the reference was successfully updated
/// * `Err(CrustError)` - If writing the reference fails
///
/// # Errors
/// * `RepositoryError` - If the reference path is invalid or write fails
pub fn update_ref(ref_name: &str, hash: &str) -> Result<(), CrustError> {
    let ref_path = Path::new(".crust").join(ref_name);

    // Ensure parent directories exist
    if let Some(parent) = ref_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the hash followed by newline
    fs::write(ref_path, format!("{}\n", hash))?;

    Ok(())
}
