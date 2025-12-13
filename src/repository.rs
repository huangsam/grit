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
