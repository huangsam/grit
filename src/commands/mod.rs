//! # Porcelain Commands Module
//!
//! This module contains implementations of high-level Git porcelain commands
//! that provide user-friendly interfaces to Grit's functionality. Porcelain
//! commands build on top of the plumbing operations to offer intuitive
//! version control operations.
//!
//! ## Overview
//!
//! Porcelain commands are the user-facing commands that most developers interact
//! with daily. Unlike plumbing operations, these commands:
//! - Provide human-readable output
//! - Handle user input validation
//! - Integrate multiple low-level operations
//! - Respect user preferences (like ignore files)
//!
//! ## Commands
//!
//! - [add]: Stage files for commit
//! - [status]: Show working directory status
//! - [reset]: Reset to previous states
//! - [diff]: Show differences between commits/files
//!
//! Each command module parses arguments, validates input, calls plumbing operations,
//! and formats results for users.
//!
//! ## Integration
//!
//! Commands integrate with repository management, index operations, object database,
//! and ignore pattern processing.

pub mod add;
pub mod diff;
pub mod reset;
pub mod status;
