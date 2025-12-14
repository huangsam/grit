//! Plumbing operations for Grit, a high-performance Git implementation in Rust.
//!
//! This module contains the core plumbing functionalities that interact directly
//! with Git objects, trees, commits, and the index. It provides efficient
//! mechanisms for reading, writing, and manipulating Git data structures.

pub mod checkout;
pub mod commits;
pub mod index;
pub mod objects;
pub mod trees;
