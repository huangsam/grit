//! # Grit - A High-Performance Git Implementation
//!
//! Grit is a from-scratch implementation of Git's core operations in Rust,
//! designed for maximum performance and minimal dependencies. Grit implements
//! both plumbing (low-level) and porcelain (high-level) commands, providing
//! a complete Git-compatible version control system.
//!
//! ## Architecture Overview
//!
//! Grit implements Git's object model with several performance optimizations:
//!
//! - Buffered I/O operations for efficient file handling
//! - Parallel tree traversal using Rayon for multi-core utilization
//! - LRU caching system for objects, hashes, and parsed trees
//! - Advanced caching with thread-safe LRU eviction policies
//!
//! ## Core Components
//!
//! ### Plumbing Operations
//! - [objects](plumbing::objects): Git object storage and retrieval (blobs, trees, commits)
//! - [trees](plumbing::trees): Tree object creation and manipulation
//! - [commits](plumbing::commits): Commit object creation and history management
//! - [checkout](plumbing::checkout): Working directory restoration from snapshots
//! - [index](plumbing::index): Git index (staging area) implementation
//!
//! ### Porcelain Commands
//! - [add](commands::add): Stage files for commit (`grit add`)
//! - [status](commands::status): Show working directory status (`grit status`)
//! - [reset](commands::reset): Reset HEAD and working directory (`grit reset`)
//!
//! ### Infrastructure
//! - [repository]: Repository initialization and management
//! - [error]: Comprehensive error handling and reporting
//! - [cache]: High-performance LRU caching system
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use grit::repository::initialize_repo;
//! use grit::plumbing::objects::{store_object, ObjectType};
//! use std::path::Path;
//!
//! // Initialize a new repository
//! initialize_repo(Path::new("/tmp/my-repo")).unwrap();
//!
//! // Store a file as a Git object
//! let content = b"Hello, World!";
//! let hash = store_object(content, ObjectType::Blob, Path::new("/tmp/my-repo")).unwrap();
//! println!("Stored object: {}", hash);
//! ```
//!
//! ## Command Line Interface
//!
//! Grit provides both plumbing and porcelain commands:
//!
//! ```bash
//! # Repository management
//! grit init
//!
//! # Porcelain commands (user-friendly)
//! grit add file.txt
//! grit status
//! grit reset --hard HEAD~1
//!
//! # Plumbing commands (low-level)
//! grit hash-object file.txt
//! grit cat-file -p <hash>
//! grit write-tree
//! grit commit -m "message"
//! grit checkout <commit>
//! grit log --oneline
//! ```
//!
//! ## Design Philosophy
//!
//! Grit follows Git's philosophy of separating plumbing (low-level operations)
//! from porcelain (high-level user interfaces). This design enables:
//!
//! - **Scriptability**: Plumbing commands are designed for automation
//! - **Composability**: Simple operations can be combined for complex workflows
//! - **Performance**: Focused implementation allows for aggressive optimization
//! - **Reliability**: Minimal surface area reduces bugs and edge cases
//!
//! ## Future Directions
//!
//! Grit continues to evolve with additional Git functionality:
//!
//! - **Branch Management**: Branch creation, switching, and merging
//! - **Packfile Support**: Efficient storage for large repositories
//! - **Advanced Diffing**: Enhanced file comparison and patch generation
//!
//! ## Contributing
//!
//! Grit is designed with performance and correctness as primary goals.
//! Contributions should maintain the high-performance characteristics while
//! ensuring compatibility with Git's specifications.

pub mod plumbing;
pub mod repository;
pub mod error;
pub mod cache;
pub mod commands;
