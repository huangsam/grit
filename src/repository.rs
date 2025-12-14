//! Repository management and initialization
//!
//! Contains functionality for setting up new Grit repositories,
//! managing repository structure, and handling repository-level operations.
//! Ensures proper directory layout and initial configuration.

use crate::error::GritError;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a Grit repository with its root directory.
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
}

impl Repository {
    /// Creates a new Repository instance for the given root path.
    pub fn new(root: &Path) -> Self {
        Repository {
            root: root.to_path_buf(),
        }
    }
}

/// Initializes a new Grit repository by creating the standard .grit directory structure.
/// This function sets up the basic directories and files needed for a Grit repository
/// to function, including objects storage, refs, and HEAD pointer.
///
/// # Arguments
/// * `repo_root` - The root directory of the repository to initialize
///
/// # Returns
/// - `Ok(())` if the repository was successfully initialized
/// - `Err(GritError)` if directory creation fails or the repository already exists
///
/// # Errors
/// This function will return an error if:
/// - The .grit directory already exists
/// - File system permissions prevent directory creation
/// - Any I/O operation fails during setup
pub fn initialize_repo(repo_root: &Path) -> Result<(), GritError> {
    let grit_dir = repo_root.join(".grit");

    // Check if repository already exists
    if grit_dir.exists() {
        return Err(GritError::RepositoryError(
            "Grit repository already exists".to_string(),
        ));
    }

    // Create the basic directory structure
    fs::create_dir(&grit_dir)?;
    fs::create_dir(grit_dir.join("objects"))?;
    fs::create_dir(grit_dir.join("refs"))?;
    fs::create_dir(grit_dir.join("refs").join("heads"))?;

    // Create HEAD file pointing to refs/heads/main
    fs::write(grit_dir.join("HEAD"), "ref: refs/heads/main\n")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_initialize_repo_success() {
        let test_dir = setup_test_dir();

        // Initialize repository
        let result = initialize_repo(test_dir.path());
        assert!(result.is_ok());

        // Check that directories were created
        assert!(test_dir.path().join(".grit").exists());
        assert!(test_dir.path().join(".grit").join("objects").exists());
        assert!(test_dir.path().join(".grit").join("refs").exists());
        assert!(
            test_dir
                .path()
                .join(".grit")
                .join("refs")
                .join("heads")
                .exists()
        );

        // Check HEAD file content
        let head_content = fs::read_to_string(test_dir.path().join(".grit").join("HEAD")).unwrap();
        assert_eq!(head_content, "ref: refs/heads/main\n");
    }

    #[test]
    fn test_initialize_repo_already_exists() {
        let test_dir = setup_test_dir();

        // Initialize repository first time
        let result1 = initialize_repo(test_dir.path());
        assert!(result1.is_ok());

        // Try to initialize again - should fail
        let result2 = initialize_repo(test_dir.path());
        assert!(result2.is_err());

        if let Err(GritError::RepositoryError(msg)) = result2 {
            assert_eq!(msg, "Grit repository already exists");
        } else {
            panic!("Expected RepositoryError");
        }
    }
}
