//! # Plumbing Operations Module
//!
//! This module contains the core plumbing functionalities that interact directly
//! with Git objects, trees, commits, and the index. It provides efficient
//! mechanisms for reading, writing, and manipulating Git data structures.
//!
//! ## Overview
//!
//! Plumbing operations are the low-level building blocks of Git functionality.
//! Unlike porcelain commands (user-friendly interfaces), plumbing commands
//! work directly with Git's internal data structures and are designed for
//! scripting and integration.
//!
//! ## Submodules
//!
//! - [objects]: Git object storage and retrieval with compression
//! - [trees]: Tree object creation and directory structure management
//! - [commits]: Commit creation and reference management
//! - [checkout]: Working directory restoration from snapshots
//! - [index]: Git index (staging area) implementation
//! - [ignores]: Ignore pattern processing for `.gritignore`
//! - [diff]: Core diff algorithm using Myers for file comparison
//!
//! ## Architecture
//!
//! These modules work together to provide:
//! - Object database management
//! - Repository state tracking
//! - History and branching support
//! - File system operations
//! - Performance optimizations (caching, parallelism)
//!
//! ## Usage
//!
//! Plumbing operations are typically used by porcelain commands but can be
//! accessed directly for advanced use cases or tool integration.

pub mod checkout;
pub mod commits;
pub mod diff;
pub mod ignores;
pub mod index;
pub mod objects;
pub mod trees;
