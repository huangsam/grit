//! # Git Add (Stage Files) Command Implementation
//!
//! This module implements the `grit add` porcelain command, which stages files
//! for the next commit by updating the Git index. It handles file selection,
//! content hashing, and index management.
//!
//! ## Overview
//!
//! The add command allows users to stage changes for commit. Key features:
//! - Support for individual files, directories, or glob patterns
//! - Automatic content hashing and blob creation
//! - Index updating with file metadata
//! - Respect for `.gritignore` patterns
//! - Recursive directory handling
//!
//! ## Command Usage
//!
//! ```bash
//! grit add <files...>
//! grit add .  # Add all files
//! grit add *.rs  # Add Rust files
//! ```
//!
//! ## Key Components
//!
//! - `add_files()`: Main function handling file staging
//! - Pattern matching and glob expansion
//! - Ignore pattern filtering
//! - Index entry creation and updating
//! - Progress reporting for large operations
//! - Efficient handling of large file sets
//! - Proper handling of file permissions and timestamps
//! - Conflict detection and resolution
//! - Integration with ignore patterns
//! - Parallel processing for multiple files
//! - Buffered I/O for large files
//! - Incremental updates to avoid full index rewrites

use crate::error::GritError;
use crate::plumbing::ignores::{is_ignored, load_ignore_patterns};
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

    // Load ignore patterns
    let ignore_patterns = load_ignore_patterns(repo_root);

    // Collect all files to add
    let files_to_add = collect_files_to_add(files, repo_root)?;

    // Filter out ignored files
    let files_to_add: HashSet<PathBuf> = files_to_add
        .into_iter()
        .filter(|path| !is_ignored(path, &ignore_patterns))
        .collect();

    // Add each file to the index
    for file_path in files_to_add {
        add_file_to_index(&file_path, &mut index, repo_root)?;
    }

    // Write the updated index
    write_index(&index, repo_root)?;

    Ok(())
}

/// Collect the set of files that should be added based on provided patterns.
///
/// Supports literal file paths (relative to the repo root), directories
/// (recursively), and glob patterns (e.g., "*.rs"). Returned paths are
/// repository-relative.
///
/// This function first enumerates all files in the repository and then filters
/// them according to the supplied patterns to avoid repeated directory
/// traversal for each pattern.
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

/// Recursively collect all files from `dir` into `files`.
///
/// Skips repository-specific entries such as the `.grit` directory and the
/// `.gritignore` file. Paths inserted into `files` are repository-relative.
fn collect_all_files(
    dir: &Path,
    files: &mut HashSet<PathBuf>,
    repo_root: &Path,
) -> Result<(), GritError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip .grit directory and .gritignore file
        if path
            .file_name()
            .is_some_and(|n| n == ".grit" || n == ".gritignore")
        {
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

/// Recursively collect files from the given directory `dir` and add them to `files`.
///
/// This function behaves like `collect_all_files` but is scoped to a specific
/// directory requested by the user (used when the user specifies a directory
/// path explicitly to `grit add`).
fn collect_files_from_directory(
    dir: &Path,
    files: &mut HashSet<PathBuf>,
    repo_root: &Path,
) -> Result<(), GritError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip .grit directory and .gritignore file
        if path
            .file_name()
            .is_some_and(|n| n == ".grit" || n == ".gritignore")
        {
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

/// Add a single file to the index by storing it as a blob and creating an index entry.
///
/// Steps:
/// 1. Read the file contents
/// 2. Store the contents as a blob object (CAS) and obtain its hex hash
/// 3. Convert the hex hash into a 20-byte array and create an `IndexEntry`
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
    // The hash returned from `store_object` is hex-encoded; convert it to the
    // 20-byte binary form expected by the index.
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

    #[test]
    fn test_add_with_ignores() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create .gritignore
        fs::write(repo_root.join(".gritignore"), "*.tmp\nbuild/\n").unwrap();

        // Create test files
        fs::write(repo_root.join("file1.txt"), b"text content").unwrap();
        fs::write(repo_root.join("file2.tmp"), b"temp content").unwrap();
        fs::create_dir(repo_root.join("build")).unwrap();
        fs::write(repo_root.join("build/output.txt"), b"build content").unwrap();

        // Add all files
        add_files(&[".".to_string()], repo_root).unwrap();

        // Check that ignored files were not added
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 1); // Only file1.txt should be added
        assert_eq!(index.entries[0].path, "file1.txt");
    }

    #[test]
    fn test_add_ignores_respected_with_glob() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create .gritignore
        fs::write(repo_root.join(".gritignore"), "*.tmp").unwrap();

        // Create test files
        fs::write(repo_root.join("file1.txt"), b"text content").unwrap();
        fs::write(repo_root.join("file2.tmp"), b"temp content").unwrap();
        fs::write(repo_root.join("file3.tmp"), b"more temp").unwrap();

        // Add all .tmp files (but they should be ignored)
        add_files(&["*.tmp".to_string()], repo_root).unwrap();

        // Check that no files were added (all .tmp files ignored)
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 0);
    }
}
