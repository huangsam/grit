//! Load and match .gritignore patterns
//!
//! This module handles loading ignore patterns from .gritignore files and matching them.
use std::fs;
use std::path::Path;

/// Loads ignore patterns from .gritignore files.
/// Currently supports root .gritignore; can be extended for subdirs.
pub fn load_ignore_patterns(repo_root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    let gritignore_path = repo_root.join(".gritignore");
    if gritignore_path.exists() {
        if let Ok(content) = fs::read_to_string(&gritignore_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    patterns.push(trimmed.to_string());
                }
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
        if pattern.starts_with("*.") {
            let ext = &pattern[2..];
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
