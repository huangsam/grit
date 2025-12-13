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
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex hash of the created commit object
/// * `Err(CrustError)` - If commit creation or storage fails
pub fn create_commit(tree_hash: &str, parent_hash: Option<&str>, message: &str, repo_root: &Path) -> Result<String, CrustError> {
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
    let hash = store_object(commit_content.as_bytes(), ObjectType::Commit, repo_root)?;

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
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(())` - If the reference was successfully updated
/// * `Err(CrustError)` - If writing the reference fails
///
/// # Errors
/// * `RepositoryError` - If the reference path is invalid or write fails
pub fn update_ref(ref_name: &str, hash: &str, repo_root: &Path) -> Result<(), CrustError> {
    let ref_path = repo_root.join(".crust").join(ref_name);

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
/// * `Err(CrustError)` - If reading HEAD fails
pub fn get_current_commit(repo_root: &Path) -> Result<Option<String>, CrustError> {
    let head_path = repo_root.join(".crust").join("HEAD");

    if !head_path.exists() {
        return Ok(None);
    }

    let head_content = fs::read_to_string(head_path)?;
    let head_content = head_content.trim();

    if let Some(ref_name) = head_content.strip_prefix("ref: ") {
        // Symbolic reference, read the actual ref
        let ref_path = repo_root.join(".crust").join(ref_name);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::repository::initialize_repo;
    use crate::plumbing::objects::{read_object, ObjectType};

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
            ref_name in prop::string::string_regex(r"refs/(heads|tags)/[a-zA-Z0-9_.-]{1,100}").unwrap()
        ) {
            let test_dir = setup_test_repo();
            let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

            update_ref(&ref_name, hash, test_dir.path())?;

            // Verify ref was created
            let ref_path = test_dir.path().join(".crust").join(ref_name);
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
        assert!(content.contains("author Crust User"));
        assert!(content.contains("committer Crust User"));
        assert!(content.contains("\n\nInitial commit\n"));
    }

    #[test]
    fn test_create_commit_with_parent() {
        let test_dir = setup_test_repo();

        let tree_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let parent_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let message = "Second commit";

        let commit_hash = create_commit(tree_hash, Some(parent_hash), message, test_dir.path()).unwrap();

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
        let head_content = fs::read_to_string(test_dir.path().join(".crust").join("HEAD")).unwrap();
        assert_eq!(head_content, format!("{}\n", test_hash));
    }

    #[test]
    fn test_update_ref_branch() {
        let test_dir = setup_test_repo();

        let test_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        // Update a branch reference
        update_ref("refs/heads/main", test_hash, test_dir.path()).unwrap();

        // Verify branch file content
        let branch_content = fs::read_to_string(test_dir.path().join(".crust").join("refs").join("heads").join("main")).unwrap();
        assert_eq!(branch_content, format!("{}\n", test_hash));
    }

    #[test]
    fn test_update_ref_creates_directories() {
        let test_dir = setup_test_repo();

        let test_hash = "cccccccccccccccccccccccccccccccccccccccc";

        // Update a deeply nested reference
        update_ref("refs/remotes/origin/main", test_hash, test_dir.path()).unwrap();

        // Verify directories were created and file content
        assert!(test_dir.path().join(".crust").join("refs").join("remotes").exists());
        assert!(test_dir.path().join(".crust").join("refs").join("remotes").join("origin").exists());

        let remote_content = fs::read_to_string(test_dir.path().join(".crust").join("refs").join("remotes").join("origin").join("main")).unwrap();
        assert_eq!(remote_content, format!("{}\n", test_hash));
    }
}
