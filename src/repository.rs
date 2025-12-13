use std::fs;
use std::path::Path;
use crate::error::CrustError;

/// Initializes a new Crust repository by creating the standard .crust directory structure.
/// This function sets up the basic directories and files needed for a Crust repository
/// to function, including objects storage, refs, and HEAD pointer.
///
/// # Arguments
/// * `repo_root` - The root directory of the repository to initialize
///
/// # Returns
/// - `Ok(())` if the repository was successfully initialized
/// - `Err(CrustError)` if directory creation fails or the repository already exists
///
/// # Errors
/// This function will return an error if:
/// - The .crust directory already exists
/// - File system permissions prevent directory creation
/// - Any I/O operation fails during setup
pub fn initialize_repo(repo_root: &Path) -> Result<(), CrustError> {
    let crust_dir = repo_root.join(".crust");

    // Check if repository already exists
    if crust_dir.exists() {
        return Err(CrustError::RepositoryError("Crust repository already exists".to_string()));
    }

    // Create the basic directory structure
    fs::create_dir(&crust_dir)?;
    fs::create_dir(crust_dir.join("objects"))?;
    fs::create_dir(crust_dir.join("refs"))?;
    fs::create_dir(crust_dir.join("refs").join("heads"))?;

    // Create HEAD file pointing to refs/heads/main
    fs::write(crust_dir.join("HEAD"), "ref: refs/heads/main\n")?;

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
        assert!(test_dir.path().join(".crust").exists());
        assert!(test_dir.path().join(".crust").join("objects").exists());
        assert!(test_dir.path().join(".crust").join("refs").exists());
        assert!(test_dir.path().join(".crust").join("refs").join("heads").exists());

        // Check HEAD file content
        let head_content = fs::read_to_string(test_dir.path().join(".crust").join("HEAD")).unwrap();
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

        if let Err(CrustError::RepositoryError(msg)) = result2 {
            assert_eq!(msg, "Crust repository already exists");
        } else {
            panic!("Expected RepositoryError");
        }
    }
}
