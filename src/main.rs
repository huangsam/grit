use clap::{Parser, Subcommand};
use grit::commands;
use grit::error::GritError;
use grit::plumbing::checkout::restore_snapshot;
use grit::plumbing::commits::{create_commit, get_current_commit, show_commit_log, update_ref};
use grit::plumbing::index::read_index;
use grit::plumbing::objects::{ObjectType, read_object, store_object};
use grit::plumbing::trees::write_tree_from_index;
use grit::repository::{Repository, initialize_repo};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Grit - A high-performance Git plumbing implementation in Rust
#[derive(Parser)]
#[command(name = "grit")]
#[command(about = "A high-performance Git plumbing implementation in Rust")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Grit Contributors")]
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
    /// Create a tree object from the current index
    ///
    /// Builds a tree object representing the current state of the staging area.
    /// Equivalent to `git write-tree`.
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
    /// Add files to the staging area
    Add {
        /// Files or patterns to add
        files: Vec<String>,
    },
    /// Show the status of the working directory and staging area
    Status,
    /// Reset current HEAD to the specified state
    Reset {
        /// The commit to reset to (defaults to HEAD)
        #[arg(default_value = "HEAD")]
        commit: String,

        /// Resets the index and working tree
        #[arg(long, group = "mode")]
        hard: bool,

        /// Resets the index but not the working tree (default)
        #[arg(long, group = "mode")]
        mixed: bool,

        /// Does not touch the index file or the working tree
        #[arg(long, group = "mode")]
        soft: bool,

        /// Paths to reset (if provided, mode must be mixed (default))
        #[arg(last = true)]
        paths: Vec<String>,
    },
    /// Show differences between two commits
    Diff {
        /// First commit hash
        hash_a: String,
        /// Second commit hash
        hash_b: String,
        /// Show diffstat instead of patch
        #[arg(long)]
        stat: bool,
    },
}

fn main() -> Result<(), GritError> {
    let cli = Cli::parse();

    let _: () = match cli.command {
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
            let index = read_index(Path::new("."))?;
            let hash = write_tree_from_index(&index, Path::new("."))?;
            println!("{}", hash);
        }
        Commands::Commit { message } => {
            // Step 1: Read the staging area (index) to see what files are staged for commit
            let index = read_index(Path::new("."))?;

            // Step 2: Create a tree object from the staged files in the index.
            // The tree represents the complete directory structure at this commit.
            let tree_hash = write_tree_from_index(&index, Path::new("."))?;

            // Step 3: Get the parent commit hash from HEAD to link this commit to history.
            // For the first commit, this will be None; for subsequent commits, it points
            // to the previous commit in the chain.
            let parent_hash = get_current_commit(Path::new("."))?;
            let parent_hash = parent_hash.as_deref();

            // Step 4: Create a new commit object containing the tree, parent reference,
            // and the commit message. This forms a node in the DAG (directed acyclic graph)
            // of commit history.
            let commit_hash = create_commit(&tree_hash, parent_hash, &message, Path::new("."))?;

            // Step 5: Update the current reference (branch or HEAD) to point to the newly
            // created commit. This advances the branch pointer forward and makes the new
            // commit part of the repository history.
            let head_path = Path::new(".grit").join("HEAD");
            let head_content = fs::read_to_string(&head_path)?;
            let current_ref = if let Some(ref_name) = head_content.strip_prefix("ref: ") {
                ref_name.trim().to_string()
            } else {
                "HEAD".to_string()
            };
            update_ref(&current_ref, &commit_hash, Path::new("."))?;

            // Output the hash of the created commit for confirmation
            println!("{}", commit_hash);
        }
        Commands::Checkout { hash } => {
            restore_snapshot(&hash, Path::new("."))?;
            println!("Restored snapshot {}", &hash[..8]);
        }
        Commands::Log { commit, oneline } => {
            show_commit_log(&commit, oneline, Path::new("."))?;
        }
        Commands::Add { files } => {
            commands::add::add_files(&files, Path::new("."))?;
        }
        Commands::Status => {
            commands::status::show_status(Path::new("."))?;
        }
        Commands::Reset {
            commit,
            hard,
            mixed: _,
            soft,
            paths,
        } => {
            if !paths.is_empty() {
                if hard || soft {
                    return Err(GritError::repo("Cannot use --hard or --soft with paths"));
                }

                let commit_hash = if commit == "HEAD" {
                    get_current_commit(Path::new("."))?.ok_or(GritError::no_commits())?
                } else {
                    commit
                };

                commands::reset::reset_paths(&commit_hash, &paths, Path::new("."))?;
            } else {
                let mode = if hard {
                    commands::reset::ResetMode::Hard
                } else if soft {
                    commands::reset::ResetMode::Soft
                } else {
                    commands::reset::ResetMode::Mixed
                };

                let commit_hash = if commit == "HEAD" {
                    get_current_commit(Path::new("."))?.ok_or(GritError::no_commits())?
                } else {
                    commit
                };

                commands::reset::reset(&commit_hash, mode, Path::new("."))?;
            }
        }
        Commands::Diff {
            hash_a,
            hash_b,
            stat,
        } => {
            let repo = Repository::new(Path::new("."));
            commands::diff::run_diff_command(&repo, &hash_a, &hash_b, stat)?;
        }
    };
    Ok(())
}
