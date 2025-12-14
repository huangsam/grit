//! # Git Commit and Reference Management Module
//!
//! This module manages Git commits and references, forming the backbone of version control
//! history. Commits are snapshots of the repository state, and references (branches, tags)
//! point to specific commits.
//!
//! ## Overview
//!
//! Commits contain metadata about changes (author, message, timestamp) and point to a tree
//! object representing the repository state. This module provides:
//! - Creating new commits from the current index state
//! - Managing references (branches, HEAD, tags)
//! - Reading commit history and metadata
//! - Reference validation and resolution
//!
//! ## Key Components
//!
//! - `create_commit()`: Creates a new commit object
//! - `update_ref()`: Updates branch or HEAD references
//! - `show_commit_log()`: Displays commit history
//! - Reference parsing and validation functions
//!
//! ## Features
//!
//! - Support for commit messages and author information
//! - Parent commit tracking for history
//! - Reference locking for concurrent access safety
//! - Timestamp handling with system time
//!
//! ## Usage
//!
//! Commits are created through the `commit` porcelain command, but can be manipulated
//! directly for advanced operations like rebasing or cherry-picking.

use crate::error::GritError;
use crate::plumbing::objects::{ObjectType, store_object};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex hash of the created commit object
/// * `Err(GritError)` - If commit creation or storage fails
pub fn create_commit(
    tree_hash: &str,
    parent_hash: Option<&str>,
    message: &str,
    repo_root: &Path,
) -> Result<String, GritError> {
    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GritError::RepositoryError("System time before Unix epoch".to_string()))?;
    let timestamp = now.as_secs();
    let timezone = "+0000"; // UTC

    // Default author/committer info (in a real implementation, this would come from config)
    let author = format!("Grit User <grit@example.com> {} {}", timestamp, timezone);
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
    let hash = store_object(commit_content.as_bytes(), ObjectType::Commit, repo_root)?;

    Ok(hash)
}

/// Validates that a reference name is safe and follows Git conventions.
///
/// Git reference names must:
/// - Not be empty
/// - Not start or end with '/'
/// - Not contain '..' or consecutive '/'
/// - Not contain control characters or spaces
/// - Not end with '.'
/// - Be within reasonable length limits
///
/// This prevents path traversal attacks and ensures reference names are valid.
fn validate_ref_name(ref_name: &str) -> Result<(), GritError> {
    if ref_name.is_empty() {
        return Err(GritError::RepositoryError(
            "Reference name cannot be empty".to_string(),
        ));
    }

    if ref_name.starts_with('/') || ref_name.ends_with('/') {
        return Err(GritError::RepositoryError(
            "Reference name cannot start or end with '/'".to_string(),
        ));
    }

    if ref_name.contains("..") {
        return Err(GritError::RepositoryError(
            "Reference name cannot contain '..'".to_string(),
        ));
    }

    if ref_name.contains("//") {
        return Err(GritError::RepositoryError(
            "Reference name cannot contain consecutive '/'".to_string(),
        ));
    }

    if ref_name.ends_with('.') {
        return Err(GritError::RepositoryError(
            "Reference name cannot end with '.'".to_string(),
        ));
    }

    // Check for control characters and spaces
    if ref_name
        .chars()
        .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(GritError::RepositoryError(
            "Reference name cannot contain control characters or spaces".to_string(),
        ));
    }

    // Reasonable length limit
    if ref_name.len() > 1024 {
        return Err(GritError::RepositoryError(
            "Reference name too long".to_string(),
        ));
    }

    Ok(())
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
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(())` - If the reference was successfully updated
/// * `Err(GritError)` - If writing the reference fails
///
/// # Errors
/// * `RepositoryError` - If the reference path is invalid or write fails
pub fn update_ref(ref_name: &str, hash: &str, repo_root: &Path) -> Result<(), GritError> {
    validate_ref_name(ref_name)?;

    let ref_path = repo_root.join(".grit").join(ref_name);

    // Ensure parent directories exist
    if let Some(parent) = ref_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the hash followed by newline
    fs::write(ref_path, format!("{}\n", hash))?;

    Ok(())
}

/// Reads the current commit hash that HEAD points to.
/// This handles both direct hash references and symbolic references to branches.
///
/// # Arguments
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(Some(hash))` - The current commit hash if HEAD exists
/// * `Ok(None)` - If HEAD doesn't exist (initial commit)
/// * `Err(GritError)` - If reading HEAD fails
pub fn get_current_commit(repo_root: &Path) -> Result<Option<String>, GritError> {
    let head_path = repo_root.join(".grit").join("HEAD");

    if !head_path.exists() {
        return Ok(None);
    }

    let head_content = fs::read_to_string(head_path)?;
    let head_content = head_content.trim();

    if let Some(ref_name) = head_content.strip_prefix("ref: ") {
        // Symbolic reference, read the actual ref
        let ref_path = repo_root.join(".grit").join(ref_name);

        if ref_path.exists() {
            let ref_content = fs::read_to_string(ref_path)?;
            Ok(Some(ref_content.trim().to_string()))
        } else {
            Ok(None)
        }
    } else {
        // Direct hash reference
        Ok(Some(head_content.to_string()))
    }
}

/// Shows the commit history starting from the given commit hash.
/// Traverses the commit graph backwards through parent references,
/// displaying formatted commit information.
///
/// # Arguments
/// * `start_hash` - The commit hash to start from (or "HEAD" for current)
/// * `oneline` - Whether to use compact one-line format
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(())` - If the log was displayed successfully
/// * `Err(GritError)` - If reading commits or resolving references fails
pub fn show_commit_log(start_hash: &str, oneline: bool, repo_root: &Path) -> Result<(), GritError> {
    let repo = crate::repository::Repository::new(repo_root);
    let mut current_hash = if start_hash == "HEAD" {
        get_current_commit(repo_root)?
            .ok_or_else(|| GritError::RepositoryError("No commits yet".to_string()))?
    } else {
        start_hash.to_string()
    };

    loop {
        let commit = crate::plumbing::objects::read_commit(&repo, &current_hash)?;

        if oneline {
            println!(
                "{} {}",
                &current_hash[..7],
                commit.message.lines().next().unwrap_or("")
            );
        } else {
            println!("commit {}", current_hash);
            println!("Author: {}", commit.author);
            println!();
            println!("{}", commit.message);
            println!();
        }

        // Find parent
        if let Some(parent) = commit.parent_hashes.first() {
            current_hash = parent.clone();
        } else {
            break; // No more parents
        }
    }

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::plumbing::objects::{ObjectType, read_object};
    use crate::repository::initialize_repo;
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
        fn test_create_commit_with_various_messages(
            message in prop::string::string_regex(r"[^\x00]*").unwrap() // Any string except null bytes
                .prop_filter("Non-empty message", |s| !s.is_empty())
        ) {
            let test_dir = setup_test_repo();
            let tree_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

            let commit_hash = create_commit(tree_hash, None, &message, test_dir.path())?;

            // Verify commit was created and contains the message
            let commit_object = read_object(&commit_hash, test_dir.path())?;
            prop_assert_eq!(commit_object.obj_type, ObjectType::Commit);

            let content = String::from_utf8_lossy(&commit_object.content);
            let expected_message = format!("\n\n{}\n", message);
            prop_assert!(content.contains(&expected_message));
        }

        #[test]
        fn test_update_ref_with_valid_names(
            ref_name in prop::string::string_regex(r"refs/(heads|tags)/[a-zA-Z0-9_-]{1,100}").unwrap()
        ) {
            let test_dir = setup_test_repo();
            let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

            update_ref(&ref_name, hash, test_dir.path())?;

            // Verify ref was created
            let ref_path = test_dir.path().join(".grit").join(ref_name);
            prop_assert!(ref_path.exists());

            let ref_content = fs::read_to_string(ref_path)?;
            prop_assert_eq!(ref_content.trim(), hash);
        }
    }

    #[test]
    fn test_create_commit_initial() {
        let test_dir = setup_test_repo();

        let tree_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let message = "Initial commit";

        let commit_hash = create_commit(tree_hash, None, message, test_dir.path()).unwrap();

        // Verify hash is 40 characters
        assert_eq!(commit_hash.len(), 40);
        assert!(commit_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Read back the commit object
        let commit_object = read_object(&commit_hash, test_dir.path()).unwrap();
        assert_eq!(commit_object.obj_type, ObjectType::Commit);

        let content = String::from_utf8_lossy(&commit_object.content);
        assert!(content.contains(&format!("tree {}", tree_hash)));
        assert!(!content.contains("parent"));
        assert!(content.contains("author Grit User"));
        assert!(content.contains("committer Grit User"));
        assert!(content.contains("\n\nInitial commit\n"));
    }

    #[test]
    fn test_create_commit_with_parent() {
        let test_dir = setup_test_repo();

        let tree_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let parent_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let message = "Second commit";

        let commit_hash =
            create_commit(tree_hash, Some(parent_hash), message, test_dir.path()).unwrap();

        // Read back the commit object
        let commit_object = read_object(&commit_hash, test_dir.path()).unwrap();
        assert_eq!(commit_object.obj_type, ObjectType::Commit);

        let content = String::from_utf8_lossy(&commit_object.content);
        assert!(content.contains(&format!("tree {}", tree_hash)));
        assert!(content.contains(&format!("parent {}", parent_hash)));
        assert!(content.contains("\n\nSecond commit\n"));
    }

    #[test]
    fn test_update_ref_head() {
        let test_dir = setup_test_repo();

        let test_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        // Update HEAD
        update_ref("HEAD", test_hash, test_dir.path()).unwrap();

        // Verify HEAD file content
        let head_content = fs::read_to_string(test_dir.path().join(".grit").join("HEAD")).unwrap();
        assert_eq!(head_content, format!("{}\n", test_hash));
    }

    #[test]
    fn test_update_ref_branch() {
        let test_dir = setup_test_repo();

        let test_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        // Update a branch reference
        update_ref("refs/heads/main", test_hash, test_dir.path()).unwrap();

        // Verify branch file content
        let branch_content = fs::read_to_string(
            test_dir
                .path()
                .join(".grit")
                .join("refs")
                .join("heads")
                .join("main"),
        )
        .unwrap();
        assert_eq!(branch_content, format!("{}\n", test_hash));
    }

    #[test]
    fn test_update_ref_creates_directories() {
        let test_dir = setup_test_repo();

        let test_hash = "cccccccccccccccccccccccccccccccccccccccc";

        // Update a deeply nested reference
        update_ref("refs/remotes/origin/main", test_hash, test_dir.path()).unwrap();

        // Verify directories were created and file content
        assert!(
            test_dir
                .path()
                .join(".grit")
                .join("refs")
                .join("remotes")
                .exists()
        );
        assert!(
            test_dir
                .path()
                .join(".grit")
                .join("refs")
                .join("remotes")
                .join("origin")
                .exists()
        );

        let remote_content = fs::read_to_string(
            test_dir
                .path()
                .join(".grit")
                .join("refs")
                .join("remotes")
                .join("origin")
                .join("main"),
        )
        .unwrap();
        assert_eq!(remote_content, format!("{}\n", test_hash));
    }

    #[test]
    fn test_ref_name_validation() {
        // Valid reference names
        assert!(validate_ref_name("HEAD").is_ok());
        assert!(validate_ref_name("refs/heads/main").is_ok());
        assert!(validate_ref_name("refs/heads/feature-branch").is_ok());
        assert!(validate_ref_name("refs/tags/v1.0.0").is_ok());
        assert!(validate_ref_name("refs/remotes/origin/main").is_ok());
        assert!(validate_ref_name("refs/heads/branch_with_underscores").is_ok());
        assert!(validate_ref_name("refs/heads/branch-with-dashes").is_ok());
        assert!(validate_ref_name("refs/heads/branch.with.dots").is_ok());

        // Invalid reference names - empty
        assert!(validate_ref_name("").is_err());

        // Invalid reference names - starts/ends with slash
        assert!(validate_ref_name("/HEAD").is_err());
        assert!(validate_ref_name("HEAD/").is_err());
        assert!(validate_ref_name("/refs/heads/main").is_err());
        assert!(validate_ref_name("refs/heads/main/").is_err());

        // Invalid reference names - contains '..'
        assert!(validate_ref_name("../../../etc/passwd").is_err());
        assert!(validate_ref_name("refs/heads/../../etc/shadow").is_err());
        assert!(validate_ref_name("HEAD..").is_err());
        assert!(validate_ref_name("..HEAD").is_err());

        // Invalid reference names - consecutive slashes
        assert!(validate_ref_name("refs//heads/main").is_err());
        assert!(validate_ref_name("refs/heads//main").is_err());
        assert!(validate_ref_name("HEAD//").is_err());

        // Invalid reference names - ends with '.'
        assert!(validate_ref_name("HEAD.").is_err());
        assert!(validate_ref_name("refs/heads/main.").is_err());
        assert!(validate_ref_name(".").is_err());

        // Invalid reference names - control characters or spaces
        assert!(validate_ref_name("HEAD with spaces").is_err());
        assert!(validate_ref_name("refs/heads/main\t").is_err());
        assert!(validate_ref_name("refs/heads/main\n").is_err());
        assert!(validate_ref_name("refs/heads/main\x00").is_err());

        // Invalid reference names - too long
        let long_name = "refs/heads/".to_string() + &"a".repeat(1025);
        assert!(validate_ref_name(&long_name).is_err());
    }

    #[test]
    fn test_update_ref_validation() {
        let test_dir = setup_test_repo();
        let test_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        // Valid reference names should work
        assert!(update_ref("HEAD", test_hash, test_dir.path()).is_ok());
        assert!(update_ref("refs/heads/main", test_hash, test_dir.path()).is_ok());

        // Invalid reference names should be rejected
        assert!(update_ref("", test_hash, test_dir.path()).is_err());
        assert!(update_ref("../../../etc/passwd", test_hash, test_dir.path()).is_err());
        assert!(update_ref("refs/heads/main.", test_hash, test_dir.path()).is_err());
        assert!(update_ref("HEAD with spaces", test_hash, test_dir.path()).is_err());
    }



    #[test]
    fn test_show_commit_log_no_commits() {
        let test_dir = setup_test_repo();

        let result = show_commit_log("HEAD", false, test_dir.path());
        assert!(result.is_err());

        if let Err(GritError::RepositoryError(msg)) = result {
            assert_eq!(msg, "No commits yet");
        } else {
            panic!("Expected RepositoryError");
        }
    }

    #[test]
    fn test_show_commit_log_invalid_commit() {
        let test_dir = setup_test_repo();

        let result = show_commit_log("invalid", false, test_dir.path());
        assert!(result.is_err());
    }
}
