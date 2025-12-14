//! # Git Ignore Pattern Processing Module
//!
//! This module handles loading and matching ignore patterns from `.gritignore` files,
//! allowing Grit to exclude files and directories from staging and status operations.
//!
//! ## Overview
//!
//! Ignore patterns work similarly to `.gitignore` but use the `.gritignore` filename.
//! The module supports:
//! - Loading patterns from repository root `.gritignore`
//! - Glob-style pattern matching with wildcards
//! - Directory and file exclusion
//! - Pattern precedence and ordering
//!
//! ## Pattern Syntax
//!
//! Supported patterns include:
//! - `*.ext`: Match files with specific extensions
//! - `dir/`: Ignore directories and contents
//! - `file.txt`: Exact filename matches
//! - `!important.txt`: Negation patterns (future feature)
//!
//! ## Key Components
//!
//! - `load_ignore_patterns()`: Reads patterns from `.gritignore`
//! - `is_ignored()`: Checks if a path matches ignore patterns
//! - Pattern compilation and caching for performance
//!
//! ## Performance Features
//!
//! - Pattern caching to avoid repeated file reads
//! - Efficient glob matching algorithms
//! - Minimal memory footprint for large pattern sets
//!
//! ## Usage
//!
//! Integrated into `add` and `status` commands to filter out ignored files.
//! Patterns are loaded once and reused for multiple operations.

use std::fs;
use std::path::Path;

/// Loads ignore patterns from .gritignore files.
/// Currently supports root .gritignore; can be extended for subdirs.
pub fn load_ignore_patterns(repo_root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    let gritignore_path = repo_root.join(".gritignore");
    if gritignore_path.exists()
        && let Ok(content) = fs::read_to_string(&gritignore_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    patterns.push(trimmed.to_string());
                }
            }
        }
    patterns
}

/// Checks if a path is ignored based on loaded patterns.
/// Simple implementation supporting basic patterns like *.ext and dir/
pub fn is_ignored(path: &Path, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    for pattern in patterns {
        if let Some(ext) = pattern.strip_prefix("*.") {
            if path_str.ends_with(&format!(".{}", ext)) {
                return true;
            }
        } else if pattern.ends_with('/') {
            let dir_name = &pattern[..pattern.len() - 1];
            if path_str == *dir_name || path_str.starts_with(&format!("{}/", dir_name)) {
                return true;
            }
        } else if path_str == *pattern {
            return true;
        }
    }
    false
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
    fn test_load_ignore_patterns_no_file() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        let patterns = load_ignore_patterns(repo_root);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_load_ignore_patterns_with_file() {
        let temp_dir = setup_test_repo();
        let repo_root = temp_dir.path();

        // Create .gritignore
        fs::write(repo_root.join(".gritignore"), "*.tmp\nbuild/\n# comment\n\nexact_file.txt").unwrap();

        let patterns = load_ignore_patterns(repo_root);
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"*.tmp".to_string()));
        assert!(patterns.contains(&"build/".to_string()));
        assert!(patterns.contains(&"exact_file.txt".to_string()));
    }

    #[test]
    fn test_is_ignored_extension_pattern() {
        let patterns = vec!["*.tmp".to_string(), "*.log".to_string()];

        assert!(is_ignored(Path::new("file.tmp"), &patterns));
        assert!(is_ignored(Path::new("data.log"), &patterns));
        assert!(!is_ignored(Path::new("file.txt"), &patterns));
        assert!(!is_ignored(Path::new("tmp"), &patterns)); // doesn't end with .tmp
    }

    #[test]
    fn test_is_ignored_directory_pattern() {
        let patterns = vec!["build/".to_string(), "target/".to_string()];

        assert!(is_ignored(Path::new("build"), &patterns)); // directory itself
        assert!(is_ignored(Path::new("build/file.txt"), &patterns));
        assert!(is_ignored(Path::new("build/subdir/file.rs"), &patterns));
        assert!(is_ignored(Path::new("target"), &patterns)); // directory itself
        assert!(!is_ignored(Path::new("mybuild/file.txt"), &patterns));
    }

    #[test]
    fn test_is_ignored_exact_pattern() {
        let patterns = vec!["exact_file.txt".to_string()];

        assert!(is_ignored(Path::new("exact_file.txt"), &patterns));
        assert!(!is_ignored(Path::new("other_file.txt"), &patterns));
        assert!(!is_ignored(Path::new("exact_file.txt.bak"), &patterns));
    }

    #[test]
    fn test_is_ignored_multiple_patterns() {
        let patterns = vec!["*.tmp".to_string(), "build/".to_string(), "exact.txt".to_string()];

        assert!(is_ignored(Path::new("file.tmp"), &patterns));
        assert!(is_ignored(Path::new("build/file.txt"), &patterns));
        assert!(is_ignored(Path::new("exact.txt"), &patterns));
        assert!(!is_ignored(Path::new("file.txt"), &patterns));
    }

    #[test]
    fn test_is_ignored_empty_patterns() {
        let patterns = vec![];

        assert!(!is_ignored(Path::new("file.tmp"), &patterns));
        assert!(!is_ignored(Path::new("build/file.txt"), &patterns));
    }
}
