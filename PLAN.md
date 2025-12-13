# Crust (Minimal Viable Git Plumbing Clone in Rust)

## Overview

### Description
Implementation of the core "plumbing" commands of Git (Content-Addressable Storage, Merkle Trees, and DAG history) in Rust, focusing on binary I/O safety and idiomatic Rust design.

### Goals
- Implement the repository object storage mechanism (hashing, compression).
- Implement snapshotting (Tree objects) via recursive directory traversal.
- Implement history tracking (Commit objects) as a Directed Acyclic Graph (DAG).
- Strictly avoid implementing the complex Git Index (staging area) layer.

## Project Structure

```
crust/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── error.rs
│   ├── utils.rs
│   ├── repository.rs
│   └── plumbing/
│       ├── mod.rs
│       ├── objects.rs
│       ├── trees.rs
│       └── commits.rs
└── .gitignore
```

### File Responsibilities
- **main.rs**: The main entry point. Contains the CLI parsing (clap) and calls the high-level logic (the "Porcelain" wrapper).
- **error.rs**: Defines the central CrustError enum for all error handling.
- **repository.rs**: Handles simple filesystem operations like initialize_repo and checking for the .git directory.
- **plumbing/objects.rs**: Contains the core CAS logic: store_object and read_object.
- **plumbing/trees.rs**: Contains the recursive snapshot logic: make_snapshot.
- **plumbing/commits.rs**: Contains the history logic: create_commit and update_ref.

## Core Components

### Project Setup
**Goal:** Initialize project and establish robust, combined error handling.

#### Cargo.toml Setup
- Initialize new Rust project 'crust'.
- Add dependencies: 'clap' (CLI parsing), 'flate2' (zlib compression/decompression), 'sha1' (for object hashing), 'hex' (for hash display).

#### Error Handling (src/error.rs)
- Define a public enum 'CrustError' to represent all failure modes. Variants must include:
  - Io(std::io::Error)
  - ObjectNotFound(String)
  - CorruptObject(String)
  - RepositoryError(String)
- Implement the 'From<std::io::Error>' trait for 'CrustError' to enable seamless use of the '?' operator with file operations.
- Implement 'std::fmt::Display' and 'std::error::Error' traits for user-friendly error messages.

### Repository Management (src/repository.rs)
- **Public fn initialize_repo**
  - **Signature:** initialize_repo() -> Result<(), CrustError>
  - **Logic:** Create the basic '.git/' directory structure ('objects', 'refs', 'HEAD', etc.).

## Plumbing Implementation

### Object Storage (src/plumbing/objects.rs)
**Goal:** Implement Content-Addressable Storage (CAS) for Blobs, Trees, and Commits.

#### Data Model
- Define a public enum 'ObjectType { Blob, Tree, Commit }'.
- Define a public struct 'Object' { type: ObjectType, content: Vec<u8> }.

#### Public fn store_object
- **Signature:** store_object(content: &[u8], obj_type: ObjectType) -> Result<String, CrustError>
- **Logic:**
  - Construct Git Header: Concatenate type string, space, size, and single null byte (\0).
  - Calculate Hash: Calculate SHA-1 hash of (Header + Content bytes).
  - Compress: Compress (Header + Content bytes) using zlib (flate2).
  - Storage: Write compressed bytes to file path '.git/objects/<hash_prefix>/<hash_suffix>'.
  - Return: SHA-1 hash string.

#### Public fn read_object
- **Signature:** read_object(hash: &str) -> Result<Object, CrustError>
- **Logic:**
  - Read File: Locate and read compressed bytes from object file path.
  - Decompress: Decompress bytes using zlib (flate2).
  - Parse Header: Locate the first null byte (\0) in the decompressed data to separate the header from the content.
  - Extract Type: Parse the header to determine ObjectType.
  - Return: Full 'Object' struct.

### Tree Objects (src/plumbing/trees.rs)
**Goal:** Implement Merkle Tree logic to capture directory structure.

#### Tree Model
- Define a public struct 'TreeEntry' { mode: String, name: String, hash: [u8; 20] }.
- Note: The hash must be stored as raw 20 bytes ([u8; 20]), not a hex string.

#### Public fn make_snapshot
- **Signature:** make_snapshot(path: &Path) -> Result<String, CrustError>
- **Logic:**
  - Recursive Walk: Iterate through the given directory path.
  - Blob/Tree Handling:
    - For files, call 'store_object' (type Blob).
    - For directories, recursively call 'make_snapshot'.
  - Format Tree Data: Assemble all TreeEntry data into a single byte array, sorted by name. Format for each entry is: <mode> <name>\0<20_raw_byte_hash>.
  - Store: Call 'store_object' on the formatted byte array (type Tree).
  - Return: The resulting Tree hash string.

### Commit Objects and Refs (src/plumbing/commits.rs)
**Goal:** Link snapshots into a DAG and manage the repository pointer (HEAD).

#### Public fn create_commit
- **Signature:** create_commit(tree_hash: &str, parent_hash: Option<&str>, message: &str) -> Result<String, CrustError>
- **Logic:**
  - Construct Commit Text: Format the Commit object text: 'tree <tree_hash>', 'parent <parent_hash>' (if provided), 'author', 'committer', followed by a blank line and the message. Use standard time/date formatting.
  - Store: Call 'store_object' on the commit text bytes (type Commit).
  - Return: The new Commit hash string.

#### Public fn update_ref
- **Signature:** update_ref(ref_name: &str, hash: &str) -> Result<(), CrustError>
- **Logic:** Write the new hash string into the designated reference file (e.g., '.git/HEAD').

## CLI Integration (src/main.rs)
**Goal:** Integrate plumbing functions into a CLI using 'clap'.

### CLI Definition
- Use 'clap' derive macros to define a main executable with subcommands corresponding to the plumbing functions:
  - 'crust init' (Calls initialize_repo)
  - 'crust hash-object <file>' (Calls store_object)
  - 'crust cat-file <hash>' (Calls read_object)
  - 'crust write-tree' (Calls make_snapshot)
  - 'crust commit' (Calls create_commit and update_ref)

### Checkout Logic (Optional, but useful)
- Define 'fn restore_snapshot(hash: &str) -> Result<(), CrustError>'.
- **Logic:** Recursively read a Tree object (using 'read_object') and write the corresponding Blob contents back to the working directory.

### Main Logic
- Use 'clap' to parse arguments and dispatch calls to the appropriate plumbing function, handling and printing 'CrustError' results cleanly.
