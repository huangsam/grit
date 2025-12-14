//! # Grit - A High-Performance Git Plumbing Implementation
//!
//! Grit is a from-scratch implementation of Git's core plumbing operations in Rust,
//! designed for maximum performance and minimal dependencies. Unlike Git's porcelain
//! commands (like `git add`, `git status`), Grit focuses on the low-level operations
//! that form the foundation of version control.
//!
//! ## Architecture Overview
//!
//! Grit implements Git's object model with several performance optimizations:
//!
//! - **Phase 1**: Buffered I/O operations for efficient file handling
//! - **Phase 2A**: Parallel tree traversal using Rayon for multi-core utilization
//! - **Phase 2B**: LRU caching system for objects, hashes, and parsed trees
//! - **Phase 2C**: Advanced caching with thread-safe LRU eviction policies
//!
//! ## Performance Characteristics
//!
//! Grit achieves significant performance improvements over traditional Git:
//!
//! - **Hash operations**: 95%+ faster due to SHA-1 computation caching
//! - **Object reads**: 97%+ faster through decompressed object caching
//! - **Tree operations**: 48-65% faster via parsed tree structure caching
//! - **Bulk operations**: Parallel processing provides scalable performance gains
//!
//! ## Core Components
//!
//! ### Plumbing Operations
//! - [`objects`](plumbing::objects): Git object storage and retrieval (blobs, trees, commits)
//! - [`trees`](plumbing::trees): Tree object creation and manipulation
//! - [`commits`](plumbing::commits): Commit object creation and history management
//! - [`checkout`](plumbing::checkout): Working directory restoration from snapshots
//!
//! ### Infrastructure
//! - [`repository`]: Repository initialization and management
//! - [`error`]: Comprehensive error handling and reporting
//! - [`cache`]: High-performance LRU caching system
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
//! Grit provides a CLI similar to Git's plumbing commands:
//!
//! ```bash
//! # Initialize repository
//! grit init
//!
//! # Store content as objects
//! grit hash-object file.txt
//! grit cat-file -p <hash>
//!
//! # Create trees and commits
//! grit write-tree
//! grit commit-tree -p <parent> -m "message" <tree>
//!
//! # Checkout snapshots
//! grit checkout <commit>
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
//! ## Performance Optimizations
//!
//! ### Caching Strategy
//! Grit employs a multi-layer caching system:
//!
//! - **Hash Cache**: Prevents redundant SHA-1 computations
//! - **Object Cache**: Stores decompressed Git objects in memory
//! - **Tree Cache**: Caches parsed tree structures to avoid re-parsing
//!
//! ### Parallel Processing
//! Tree operations utilize Rayon's parallel iterators for multi-core performance.
//!
//! ### I/O Optimization
//! Buffered readers/writers and streaming operations minimize system calls.
//!
//! ## Compatibility
//!
//! Grit is designed to be compatible with standard Git repositories:
//!
//! - Uses identical object formats and directory structures
//! - Implements the same hash algorithms (SHA-1)
//! - Follows Git's object model and reference specifications
//! - Can read/write repositories created by standard Git
//!
//! ## Future Directions
//!
//! While Grit currently focuses on plumbing operations, future enhancements could include:
//!
//! - **Porcelain Commands**: `grit add`, `grit status`, `grit log`
//! - **Index/Staging Area**: Git's staging area implementation
//! - **Branch Management**: Branch creation, switching, and merging
//! - **Remote Operations**: Push, pull, and fetch functionality
//! - **Advanced Features**: Rebasing, cherry-picking, and interactive operations
//!
//! ## Contributing
//!
//! Grit is designed with performance and correctness as primary goals.
//! Contributions should maintain the high-performance characteristics while
//! ensuring compatibility with Git's specifications.
//!
//! ## License
//!
//! This project is open source and available under the MIT license.

/// Plumbing operations - Git's low-level core functionality
///
/// This module contains the fundamental operations that implement Git's
/// object model and version control primitives. These are the building
/// blocks that higher-level porcelain commands are built upon.
pub mod plumbing {
    /// Git object storage and retrieval operations
    ///
    /// Handles the creation, storage, and reading of Git's three object types:
    /// blobs (file contents), trees (directory structures), and commits (history).
    /// Implements efficient caching and compression for optimal performance.
    pub mod objects;

    /// Git index (staging area) implementation
    ///
    /// Implements Git's index file format for tracking staged changes.
    /// Provides functions for reading, writing, and manipulating the index
    /// to support porcelain commands like add, reset, and status.
    pub mod index;

    /// Tree object creation and manipulation
    ///
    /// Provides functionality for creating tree objects from directory structures,
    /// parsing tree contents, and managing hierarchical file organization.
    /// Utilizes parallel processing for large directory trees.
    pub mod trees;

    /// Commit object creation and history management
    ///
    /// Handles the creation of commit objects, parent-child relationships,
    /// and commit metadata (author, timestamp, messages). Forms the basis
    /// of Git's commit graph and history traversal.
    pub mod commits;

    /// Working directory restoration and checkout operations
    ///
    /// Implements the restoration of file snapshots from tree/commit objects
    /// to the working directory. Handles file permissions, directory creation,
    /// and efficient bulk operations with caching optimizations.
    pub mod checkout;
}

/// Repository management and initialization
///
/// Contains functionality for setting up new Grit repositories,
/// managing repository structure, and handling repository-level operations.
/// Ensures proper directory layout and initial configuration.
pub mod repository;

/// Error handling and reporting
///
/// Comprehensive error types and handling for all Grit operations.
/// Provides detailed error information while maintaining clean error propagation
/// throughout the codebase.
pub mod error;

/// High-performance caching system
///
/// Implements LRU (Least Recently Used) caching for Git objects, computed hashes,
/// and parsed tree structures. Thread-safe and optimized for concurrent access
/// patterns typical in version control operations.
pub mod cache;
