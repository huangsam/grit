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
//! ## Available Commands
//!
//! - [add]: Stage files for commit
//! - [status]: Show working directory status
//! - [reset]: Reset to previous states
//! - [diff]: Show differences between commits/files
//!
//! ## Architecture
//!
//! Each command module:
//! - Parses command-line arguments
//! - Validates input and repository state
//! - Calls appropriate plumbing operations
//! - Formats and displays results
//! - Handles errors gracefully
//!
//! ## Integration
//!
//! Commands integrate with:
//! - Repository management
//! - Index operations
//! - Object database
//! - Ignore pattern processing
//! - Working directory manipulation

pub mod add;
pub mod diff;
pub mod status;
pub mod reset;
