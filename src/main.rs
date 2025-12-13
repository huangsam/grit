mod error;
mod repository;
mod plumbing;

use clap::{Parser, Subcommand};
use std::path::Path;
use std::fs;
use crate::error::CrustError;
use crate::repository::initialize_repo;
use crate::plumbing::objects::{store_object, read_object, ObjectType};
use crate::plumbing::trees::make_snapshot;
use crate::plumbing::commits::{create_commit, update_ref, get_current_commit};

/// Crust - A minimal Git plumbing clone in Rust
#[derive(Parser)]
#[command(name = "crust")]
#[command(about = "Minimal plumbing implementation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Crust repository
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
}

fn main() -> Result<(), CrustError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            initialize_repo(Path::new("."))?;
            println!("Initialized empty Crust repository");
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
                    // For blobs, print the content directly
                    print!("{}", String::from_utf8_lossy(&object.content));
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
            let head_path = Path::new(".crust").join("HEAD");
            let head_content = fs::read_to_string(&head_path)?;
            let current_ref = if let Some(ref_name) = head_content.strip_prefix("ref: ") {
                ref_name.trim().to_string()
            } else {
                "HEAD".to_string()
            };
            update_ref(&current_ref, &commit_hash, Path::new("."))?;

            println!("{}", commit_hash);
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

    fn run_crust_command(test_dir: &TempDir, args: &[&str]) -> Result<String, String> {
        let crust_binary = env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("crust");

        println!("Running: {} {:?} in {:?}", crust_binary.display(), args, test_dir);

        let output = Command::new(&crust_binary)
            .args(args)
            .current_dir(test_dir)
            .output()
            .map_err(|e| format!("Failed to run command: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        let result = run_crust_command(&test_dir, &["init"]);
        assert!(result.is_ok(), "Init failed: {:?}", result);

        // Verify .crust directory exists
        assert!(test_dir.path().join(".crust").exists());
        assert!(test_dir.path().join(".crust/objects").exists());
        assert!(test_dir.path().join(".crust/refs").exists());

        // Create a test file
        fs::write(test_dir.path().join("hello.txt"), "Hello, World!").unwrap();

        // Hash the file
        let hash_result = run_crust_command(&test_dir, &["hash-object", "hello.txt"]);
        assert!(hash_result.is_ok(), "Hash-object failed: {:?}", hash_result);
        let blob_hash = hash_result.unwrap();
        assert_eq!(blob_hash.len(), 40);

        // Create tree snapshot
        let tree_result = run_crust_command(&test_dir, &["write-tree"]);
        assert!(tree_result.is_ok(), "Write-tree failed: {:?}", tree_result);
        let tree_hash = tree_result.unwrap();
        assert_eq!(tree_hash.len(), 40);

        // Create commit
        let commit_result = run_crust_command(&test_dir, &["commit", "--message", "Initial commit"]);
        assert!(commit_result.is_ok(), "Commit failed: {:?}", commit_result);
        let commit_hash = commit_result.unwrap();
        assert_eq!(commit_hash.len(), 40);

        // Verify commit object exists
        let commit_file = test_dir.path().join(".crust/objects")
            .join(&commit_hash[..2])
            .join(&commit_hash[2..]);
        assert!(commit_file.exists(), "Commit object file should exist");

        // Read the commit
        let cat_result = run_crust_command(&test_dir, &["cat-file", &commit_hash]);
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
        let _ = run_crust_command(&test_dir, &["init"]);
        let double_init = run_crust_command(&test_dir, &["init"]);
        assert!(double_init.is_err(), "Double init should fail");

        // Try to read nonexistent object
        let nonexistent = run_crust_command(&test_dir, &["cat-file", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]);
        assert!(nonexistent.is_err(), "Reading nonexistent object should fail");
    }

    #[test]
    fn test_commit_with_parent() {
        let test_dir = setup_integration_test();

        // Initialize and create first commit
        run_crust_command(&test_dir, &["init"]).unwrap();
        fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
        run_crust_command(&test_dir, &["commit", "--message", "First commit"]).unwrap();

        // Create second commit
        fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
        let commit_result = run_crust_command(&test_dir, &["commit", "--message", "Second commit"]);
        assert!(commit_result.is_ok(), "Second commit failed: {:?}", commit_result);

        // Verify HEAD points to new commit
        let head_content = fs::read_to_string(test_dir.path().join(".crust/HEAD")).unwrap();
        let head_ref = head_content.trim().strip_prefix("ref: ").unwrap();
        let branch_content = fs::read_to_string(test_dir.path().join(".crust").join(head_ref)).unwrap();
        let latest_commit = branch_content.trim();

        // Read the latest commit
        let cat_result = run_crust_command(&test_dir, &["cat-file", latest_commit]);
        assert!(cat_result.is_ok());
        let commit_content = cat_result.unwrap();
        assert!(commit_content.contains("parent"));
        assert!(commit_content.contains("Second commit"));
    }
}
