use std::fs;
use std::path::Path;
use crate::error::CrustError;

/// Initializes a new Crust repository by creating the standard .crust directory structure.
/// This function sets up the basic directories and files needed for a Crust repository
/// to function, including objects storage, refs, and HEAD pointer.
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
pub fn initialize_repo() -> Result<(), CrustError> {
    let crust_dir = Path::new(".crust");

    // Check if repository already exists
    if crust_dir.exists() {
        return Err(CrustError::RepositoryError("Crust repository already exists".to_string()));
    }

    // Create the basic directory structure
    fs::create_dir(crust_dir)?;
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
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("crust_test_repo_{}", timestamp));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();
        env::set_current_dir(&temp_dir).unwrap();
        temp_dir
    }

    fn cleanup_test_dir(test_dir: PathBuf) {
        if test_dir.exists() {
            env::set_current_dir(env::temp_dir().parent().unwrap()).unwrap();
            fs::remove_dir_all(test_dir).unwrap();
        }
    }

    #[test]
    fn test_initialize_repo_success() {
        let test_dir = setup_test_dir();

        // Initialize repository
        let result = initialize_repo();
        assert!(result.is_ok());

        // Check that directories were created
        assert!(Path::new(".crust").exists());
        assert!(Path::new(".crust/objects").exists());
        assert!(Path::new(".crust/refs").exists());
        assert!(Path::new(".crust/refs/heads").exists());

        // Check HEAD file content
        let head_content = fs::read_to_string(".crust/HEAD").unwrap();
        assert_eq!(head_content, "ref: refs/heads/main\n");

        cleanup_test_dir(test_dir);
    }

    #[test]
    fn test_initialize_repo_already_exists() {
        let test_dir = setup_test_dir();

        // Initialize repository first time
        let result1 = initialize_repo();
        assert!(result1.is_ok());

        // Try to initialize again - should fail
        let result2 = initialize_repo();
        assert!(result2.is_err());

        if let Err(CrustError::RepositoryError(msg)) = result2 {
            assert_eq!(msg, "Crust repository already exists");
        } else {
            panic!("Expected RepositoryError");
        }

        cleanup_test_dir(test_dir);
    }
}
