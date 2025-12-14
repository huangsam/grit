//! Implementation of the `grit add` porcelain command
//!
//! This module handles staging files for the next commit by updating the Git index.

use crate::error::GritError;
use crate::plumbing::index::{create_index_entry, read_index, write_index};
use crate::plumbing::objects::{ObjectType, store_object};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Add files to the staging area
///
/// This function stages files for the next commit by updating the Git index.
/// It supports individual files, directories, and glob patterns (e.g., "*.rs").
/// Files are compressed and stored as blob objects, then added to the index.
///
/// # Arguments
///
/// * `files` - A slice of file paths or patterns to add. Can include:
///   - Individual files: `["file.txt"]`
///   - Directories: `["src/"]` (adds all files recursively)
///   - Glob patterns: `["*.rs", "*.toml"]`
///   - Current directory: `["."]` (adds all files)
/// * `repo_root` - The root directory of the Git repository
///
/// # Returns
///
/// Returns `Ok(())` if all files were successfully added to the index.
///
/// # Errors
///
/// Returns `GritError` if:
/// - The repository is not initialized
/// - Files cannot be read
/// - Index cannot be read or written
/// - Invalid glob patterns are provided
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
/// use grit::commands::add::add_files;
///
/// let repo_root = Path::new("/path/to/repo");
///
/// // Add a single file
/// add_files(&["README.md".to_string()], repo_root).unwrap();
///
/// // Add all Rust files
/// add_files(&["*.rs".to_string()], repo_root).unwrap();
///
/// // Add entire directory
/// add_files(&["src/".to_string()], repo_root).unwrap();
/// ```
pub fn add_files(files: &[String], repo_root: &Path) -> Result<(), GritError> {
    // Read the current index
    let mut index = read_index(repo_root)?;

    // Collect all files to add
    let files_to_add = collect_files_to_add(files, repo_root)?;

    // Add each file to the index
    for file_path in files_to_add {
        add_file_to_index(&file_path, &mut index, repo_root)?;
    }

    // Write the updated index
    write_index(&index, repo_root)?;

    Ok(())
}

/// Collect all files that should be added based on the provided patterns/files
fn collect_files_to_add(
    patterns: &[String],
    repo_root: &Path,
) -> Result<HashSet<PathBuf>, GritError> {
    // First, collect all files in the repository
    let mut all_files = HashSet::new();
    collect_all_files(repo_root, &mut all_files, repo_root)?;

    let mut files = HashSet::new();

    for pattern in patterns {
        if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            // Glob pattern
            let glob_pattern = glob::Pattern::new(pattern)
                .map_err(|e| GritError::RepositoryError(format!("Invalid glob pattern: {}", e)))?;

            for file in &all_files {
                if glob_pattern.matches_path(file) {
                    files.insert(file.clone());
                }
            }
        } else {
            // Specific file or directory
            let path = repo_root.join(pattern);
            if path.is_dir() {
                collect_files_from_directory(&path, &mut files, repo_root)?;
            } else if path.is_file()
                && let Ok(relative) = path.strip_prefix(repo_root)
            {
                files.insert(relative.to_path_buf());
            }
            // If not found, ignore (like Git does)
        }
    }

    Ok(files)
}

/// Recursively collect all files from the repository
fn collect_all_files(
    dir: &Path,
    files: &mut HashSet<PathBuf>,
    repo_root: &Path,
) -> Result<(), GritError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip .grit directory
        if path.file_name().is_some_and(|n| n == ".grit") {
            continue;
        }

        if path.is_dir() {
            collect_all_files(&path, files, repo_root)?;
        } else if path.is_file()
            && let Ok(relative) = path.strip_prefix(repo_root)
        {
            files.insert(relative.to_path_buf());
        }
    }

    Ok(())
}

/// Recursively collect all files from a directory
fn collect_files_from_directory(
    dir: &Path,
    files: &mut HashSet<PathBuf>,
    repo_root: &Path,
) -> Result<(), GritError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip .grit directory
        if path.file_name().is_some_and(|n| n == ".grit") {
            continue;
        }

        if path.is_dir() {
            collect_files_from_directory(&path, files, repo_root)?;
        } else if path.is_file()
            && let Ok(relative) = path.strip_prefix(repo_root)
        {
            files.insert(relative.to_path_buf());
        }
    }

    Ok(())
}

/// Add a single file to the index
fn add_file_to_index(
    file_path: &Path,
    index: &mut crate::plumbing::index::Index,
    repo_root: &Path,
) -> Result<(), GritError> {
    let full_path = repo_root.join(file_path);

    // Read file content
    let content = fs::read(&full_path)?;

    // Store as blob object
    let hash = store_object(&content, ObjectType::Blob, repo_root)?;

    // Parse hash from hex string to bytes
    let hash_bytes = hex::decode(&hash).map_err(|_| {
        GritError::RepositoryError("Invalid hash returned from store_object".to_string())
    })?;

    let mut hash_array = [0u8; 20];
    hash_array.copy_from_slice(&hash_bytes);

    // Create index entry
    let entry = create_index_entry(&full_path, &hash_array, repo_root)?;

    // Add to index
    index.add_entry(entry);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Initialize repository
        crate::repository::initialize_repo(repo_root).unwrap();

        temp_dir
    }

    #[test]
    fn test_add_single_file() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create a test file
        let file_path = repo_root.join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        // Add the file
        add_files(&["test.txt".to_string()], repo_root).unwrap();

        // Check that it was added to index
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].path, "test.txt");
    }

    #[test]
    fn test_add_directory() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create directory structure
        fs::create_dir(repo_root.join("subdir")).unwrap();
        fs::write(repo_root.join("file1.txt"), b"content1").unwrap();
        fs::write(repo_root.join("subdir/file2.txt"), b"content2").unwrap();

        // Add the directory
        add_files(&[".".to_string()], repo_root).unwrap();

        // Check that files were added
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 2);
        let paths: std::collections::HashSet<_> =
            index.entries.iter().map(|e| e.path.clone()).collect();
        assert!(paths.contains("file1.txt"));
        assert!(paths.contains("subdir/file2.txt"));
    }

    #[test]
    fn test_add_glob_pattern() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create test files
        fs::write(repo_root.join("file1.rs"), b"rust code").unwrap();
        fs::write(repo_root.join("file2.rs"), b"more rust").unwrap();
        fs::write(repo_root.join("file3.txt"), b"text file").unwrap();

        // Add .rs files
        add_files(&["*.rs".to_string()], repo_root).unwrap();

        // Check that only .rs files were added
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 2);
        let paths: std::collections::HashSet<_> =
            index.entries.iter().map(|e| e.path.clone()).collect();
        assert!(paths.contains("file1.rs"));
        assert!(paths.contains("file2.rs"));
        assert!(!paths.contains("file3.txt"));
    }
}
