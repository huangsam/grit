mod error;
mod utils;
mod repository;
mod plumbing;

use clap::{Parser, Subcommand};
use std::path::Path;
use crate::error::CrustError;
use crate::repository::initialize_repo;
use crate::plumbing::objects::{store_object, read_object, ObjectType};
use crate::plumbing::trees::make_snapshot;
use crate::plumbing::commits::{create_commit, update_ref};

/// Crust - A minimal Git plumbing clone in Rust
#[derive(Parser)]
#[command(name = "crust")]
#[command(about = "Minimal plumbing implementation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
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
            initialize_repo()?;
            println!("Initialized empty Crust repository");
        }
        Commands::HashObject { file } => {
            let path = Path::new(&file);
            let content = std::fs::read(path)?;
            let hash = store_object(&content, ObjectType::Blob)?;
            println!("{}", hash);
        }
        Commands::CatFile { hash } => {
            let object = read_object(&hash)?;
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
            let hash = make_snapshot(Path::new("."))?;
            println!("{}", hash);
        }
        Commands::Commit { message } => {
            // For now, assume we have a tree from write-tree
            // In a real implementation, we'd track the current tree
            let tree_hash = "dummy_tree_hash"; // TODO: Get actual tree hash
            let parent_hash = None; // TODO: Get parent from HEAD
            let commit_hash = create_commit(tree_hash, parent_hash, &message)?;
            update_ref("HEAD", &commit_hash)?;
            println!("{}", commit_hash);
        }
    }

    Ok(())
}
