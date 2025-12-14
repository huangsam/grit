mod error;
mod repository;
mod plumbing;
mod cache;

use clap::{Parser, Subcommand};
use std::path::Path;
use std::fs;
use std::io::Write;
use crate::error::GritError;
use crate::repository::initialize_repo;
use crate::plumbing::objects::{store_object, read_object, ObjectType};
use crate::plumbing::trees::make_snapshot;
use crate::plumbing::commits::{create_commit, update_ref, get_current_commit, show_commit_log};
use crate::plumbing::checkout::restore_snapshot;

/// Grit - A minimal Git plumbing clone in Rust
#[derive(Parser)]
#[command(name = "grit")]
#[command(about = "Minimal plumbing implementation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Grit repository
    Init,
    /// Store a file in the object database
    HashObject {
        /// Path to the file to store
        file: String,
    },
    /// Display the contents of an object
    CatFile {
        /// Hash of the object to display
        hash: String,
    },
    /// Create a tree object from the current directory
    WriteTree,
    /// Create a commit object
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },
    /// Restore a tree or commit snapshot to the working directory
    Checkout {
        /// Hash of tree or commit to restore
        hash: String,
    },
    /// Show commit history
    Log {
        /// Commit hash to start from (defaults to HEAD)
        #[arg(default_value = "HEAD")]
        commit: String,
        /// Show compact one-line format
        #[arg(short, long)]
        oneline: bool,
    },
}

fn main() -> Result<(), GritError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            initialize_repo(Path::new("."))?;
            println!("Initialized empty Grit repository");
        }
        Commands::HashObject { file } => {
            let path = Path::new(&file);
            let content = std::fs::read(path)?;
            let hash = store_object(&content, ObjectType::Blob, Path::new("."))?;
            println!("{}", hash);
        }
        Commands::CatFile { hash } => {
            let object = read_object(&hash, Path::new("."))?;
            match object.obj_type {
                ObjectType::Blob => {
                    // For blobs, output the raw content
                    std::io::stdout().write_all(&object.content)?;
                }
                ObjectType::Tree | ObjectType::Commit => {
                    // For trees and commits, print as UTF-8 text
                    println!("{}", String::from_utf8_lossy(&object.content));
                }
            }
        }
        Commands::WriteTree => {
            let hash = make_snapshot(Path::new("."), Path::new("."))?;
            println!("{}", hash);
        }
        Commands::Commit { message } => {
            // Get the current tree snapshot
            let tree_hash = make_snapshot(Path::new("."), Path::new("."))?;

            // Get the parent commit from HEAD
            let parent_hash = get_current_commit(Path::new("."))?;
            let parent_hash = parent_hash.as_deref();

            // Create the commit
            let commit_hash = create_commit(&tree_hash, parent_hash, &message, Path::new("."))?;

            // Update the current branch or HEAD to point to the new commit
            let head_path = Path::new(".grit").join("HEAD");
            let head_content = fs::read_to_string(&head_path)?;
            let current_ref = if let Some(ref_name) = head_content.strip_prefix("ref: ") {
                ref_name.trim().to_string()
            } else {
                "HEAD".to_string()
            };
            update_ref(&current_ref, &commit_hash, Path::new("."))?;

            println!("{}", commit_hash);
        }
        Commands::Checkout { hash } => {
            restore_snapshot(&hash, Path::new("."))?;
            println!("Restored snapshot {}", &hash[..8]);
        }
        Commands::Log { commit, oneline } => {
            show_commit_log(&commit, oneline, Path::new("."))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_integration_test() -> TempDir {
        TempDir::new().unwrap()
    }

    fn run_grit_command(test_dir: &TempDir, args: &[&str]) -> Result<String, String> {
        let grit_binary = env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("grit");

        println!("Running: {} {:?} in {:?}", grit_binary.display(), args, test_dir);

        let output = Command::new(&grit_binary)
            .args(args)
            .current_dir(test_dir)
            .output()
            .map_err(|e| format!("Failed to run command: {}", e))?;

        if output.status.success() {
            // For cat-file command, don't trim to preserve exact content
            if args.get(0) == Some(&"cat-file") {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("stdout: {}\nstderr: {}", stdout, stderr))
        }
    }

    #[test]
    fn test_full_git_workflow() {
        let test_dir = setup_integration_test();

        // Initialize repository
        let result = run_grit_command(&test_dir, &["init"]);
        assert!(result.is_ok(), "Init failed: {:?}", result);

        // Verify .grit directory exists
        assert!(test_dir.path().join(".grit").exists());
        assert!(test_dir.path().join(".grit/objects").exists());
        assert!(test_dir.path().join(".grit/refs").exists());

        // Create a test file
        fs::write(test_dir.path().join("hello.txt"), "Hello, World!").unwrap();

        // Hash the file
        let hash_result = run_grit_command(&test_dir, &["hash-object", "hello.txt"]);
        assert!(hash_result.is_ok(), "Hash-object failed: {:?}", hash_result);
        let blob_hash = hash_result.unwrap();
        assert_eq!(blob_hash.len(), 40);

        // Create tree snapshot
        let tree_result = run_grit_command(&test_dir, &["write-tree"]);
        assert!(tree_result.is_ok(), "Write-tree failed: {:?}", tree_result);
        let tree_hash = tree_result.unwrap();
        assert_eq!(tree_hash.len(), 40);

        // Create commit
        let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Initial commit"]);
        assert!(commit_result.is_ok(), "Commit failed: {:?}", commit_result);
        let commit_hash = commit_result.unwrap();
        assert_eq!(commit_hash.len(), 40);

        // Verify commit object exists
        let commit_file = test_dir.path().join(".grit/objects")
            .join(&commit_hash[..2])
            .join(&commit_hash[2..]);
        assert!(commit_file.exists(), "Commit object file should exist");

        // Read the commit
        let cat_result = run_grit_command(&test_dir, &["cat-file", &commit_hash]);
        assert!(cat_result.is_ok(), "Cat-file failed: {:?}", cat_result);
        let commit_content = cat_result.unwrap();
        assert!(commit_content.contains("tree"));
        assert!(commit_content.contains("author"));
        assert!(commit_content.contains("committer"));
        assert!(commit_content.contains("Initial commit"));
    }

    #[test]
    fn test_error_cases() {
        let test_dir = setup_integration_test();

        // Try to initialize twice
        let _ = run_grit_command(&test_dir, &["init"]);
        let double_init = run_grit_command(&test_dir, &["init"]);
        assert!(double_init.is_err(), "Double init should fail");

        // Try to read nonexistent object
        let nonexistent = run_grit_command(&test_dir, &["cat-file", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]);
        assert!(nonexistent.is_err(), "Reading nonexistent object should fail");
    }

    #[test]
    fn test_commit_with_parent() {
        let test_dir = setup_integration_test();

        // Initialize and create first commit
        run_grit_command(&test_dir, &["init"]).unwrap();
        fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
        run_grit_command(&test_dir, &["commit", "--message", "First commit"]).unwrap();

        // Create second commit
        fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
        let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Second commit"]);
        assert!(commit_result.is_ok(), "Second commit failed: {:?}", commit_result);

        // Verify HEAD points to new commit
        let head_content = fs::read_to_string(test_dir.path().join(".grit/HEAD")).unwrap();
        let head_ref = head_content.trim().strip_prefix("ref: ").unwrap();
        let branch_content = fs::read_to_string(test_dir.path().join(".grit").join(head_ref)).unwrap();
        let latest_commit = branch_content.trim();

        // Read the latest commit
        let cat_result = run_grit_command(&test_dir, &["cat-file", latest_commit]);
        assert!(cat_result.is_ok());
        let commit_content = cat_result.unwrap();
        assert!(commit_content.contains("parent"));
        assert!(commit_content.contains("Second commit"));
    }

    #[test]
    fn test_log_command() {
        let test_dir = setup_integration_test();

        // Initialize and create commits
        run_grit_command(&test_dir, &["init"]).unwrap();
        fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
        run_grit_command(&test_dir, &["commit", "--message", "First commit"]).unwrap();

        fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
        run_grit_command(&test_dir, &["commit", "--message", "Second commit"]).unwrap();

        // Test log command
        let log_result = run_grit_command(&test_dir, &["log"]);
        assert!(log_result.is_ok(), "Log command failed: {:?}", log_result);
        let log_output = log_result.unwrap();
        assert!(log_output.contains("commit"));
        assert!(log_output.contains("Author:"));
        assert!(log_output.contains("Second commit"));
        assert!(log_output.contains("First commit"));

        // Test oneline format
        let oneline_result = run_grit_command(&test_dir, &["log", "--oneline"]);
        assert!(oneline_result.is_ok(), "Oneline log failed: {:?}", oneline_result);
        let oneline_output = oneline_result.unwrap();
        assert!(oneline_output.lines().count() == 2); // Two commits
        assert!(oneline_output.contains("Second commit"));
        assert!(oneline_output.contains("First commit"));
    }

    #[test]
    fn test_log_command_single_commit() {
        let test_dir = setup_integration_test();

        // Initialize and create one commit
        run_grit_command(&test_dir, &["init"]).unwrap();
        fs::write(test_dir.path().join("file.txt"), "Content").unwrap();
        run_grit_command(&test_dir, &["commit", "--message", "Single commit"]).unwrap();

        // Test log command
        let log_result = run_grit_command(&test_dir, &["log"]);
        assert!(log_result.is_ok());
        let log_output = log_result.unwrap();
        assert!(log_output.contains("commit"));
        assert!(log_output.contains("Author:"));
        assert!(log_output.contains("Single commit"));
        assert!(!log_output.contains("parent")); // No parent for first commit
    }

    #[test]
    fn test_log_command_error_cases() {
        let test_dir = setup_integration_test();

        // Test log without repo
        let log_result = run_grit_command(&test_dir, &["log"]);
        assert!(log_result.is_err(), "Log should fail without commits");

        // Initialize but no commits
        run_grit_command(&test_dir, &["init"]).unwrap();
        let log_result = run_grit_command(&test_dir, &["log"]);
        assert!(log_result.is_err(), "Log should fail without commits");

        // Test invalid commit hash
        let log_result = run_grit_command(&test_dir, &["log", "invalidhash"]);
        assert!(log_result.is_err(), "Log should fail with invalid hash");
    }

    #[test]
    fn test_checkout_workflow() {
        let test_dir = setup_integration_test();

        // Initialize repository
        run_grit_command(&test_dir, &["init"]).unwrap();

        // Create files and commit
        fs::write(test_dir.path().join("original.txt"), "Original content").unwrap();
        fs::create_dir(test_dir.path().join("subdir")).unwrap();
        fs::write(test_dir.path().join("subdir").join("nested.txt"), "Nested content").unwrap();

        let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Initial commit"]);
        assert!(commit_result.is_ok());
        let commit_hash = commit_result.unwrap();

        // Modify files
        fs::write(test_dir.path().join("original.txt"), "Modified content").unwrap();
        fs::remove_file(test_dir.path().join("subdir").join("nested.txt")).unwrap();

        // Checkout the commit
        let checkout_result = run_grit_command(&test_dir, &["checkout", &commit_hash]);
        assert!(checkout_result.is_ok());

        // Verify files were restored
        assert_eq!(fs::read_to_string(test_dir.path().join("original.txt")).unwrap(), "Original content");
        assert!(test_dir.path().join("subdir").join("nested.txt").exists());
        assert_eq!(fs::read_to_string(test_dir.path().join("subdir").join("nested.txt")).unwrap(), "Nested content");
    }

    #[test]
    fn test_git_compatibility() {
        let test_dir = setup_integration_test();

        // Initialize grit repository
        run_grit_command(&test_dir, &["init"]).unwrap();

        // Create test files with various content types
        let test_files = vec![
            ("empty.txt", ""),
            ("simple.txt", "Hello World"),
            ("unicode.txt", "🚀 Hello 世界 🌍"),
            ("multiline.txt", "Line 1\nLine 2\nLine 3\n"),
        ];

        for (filename, content) in test_files {
            fs::write(test_dir.path().join(filename), content).unwrap();

            // Get grit hash
            let grit_hash = run_grit_command(&test_dir, &["hash-object", filename]).unwrap();

            // Get git hash for comparison
            let git_output = Command::new("git")
                .args(&["hash-object", filename])
                .current_dir(test_dir.path())
                .output()
                .expect("git command failed");

            assert!(git_output.status.success(), "git hash-object failed");
            let git_hash = String::from_utf8_lossy(&git_output.stdout).trim().to_string();

            // Compare hashes
            assert_eq!(grit_hash, git_hash,
                "Hash mismatch for file {}: grit={}, git={}", filename, grit_hash, git_hash);

            // Verify we can read the object back
            let cat_result = run_grit_command(&test_dir, &["cat-file", &grit_hash]);
            assert!(cat_result.is_ok(), "Failed to read object {}", grit_hash);
            let read_content = cat_result.unwrap();

            // Compare content
            assert_eq!(read_content, content);
        }

        // Test binary file separately
        let binary_content = vec![0, 1, 255, 128];
        fs::write(test_dir.path().join("binary.dat"), &binary_content).unwrap();

        let grit_hash = run_grit_command(&test_dir, &["hash-object", "binary.dat"]).unwrap();

        let git_output = Command::new("git")
            .args(&["hash-object", "binary.dat"])
            .current_dir(test_dir.path())
            .output()
            .expect("git command failed");

        assert!(git_output.status.success(), "git hash-object failed for binary file");
        let git_hash = String::from_utf8_lossy(&git_output.stdout).trim().to_string();

        assert_eq!(grit_hash, git_hash,
            "Hash mismatch for binary file: grit={}, git={}", grit_hash, git_hash);

        // For binary content, compare by reading the object directly instead of through cat-file
        let read_obj = crate::plumbing::objects::read_object(&grit_hash, test_dir.path()).unwrap();
        assert_eq!(read_obj.obj_type, crate::plumbing::objects::ObjectType::Blob);
        assert_eq!(read_obj.content, binary_content);
    }
}
