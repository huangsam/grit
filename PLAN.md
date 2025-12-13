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
- **Purpose:** The core function to read any content, calculate its hash, compress it, and save it to the .git/objects directory.
- **Steps:**
  - **Step 1: Header Construction** - Prepend the content with the exact Git header: `<ASCII type> <ASCII size>\0`. Example: `b"blob 12\0"`.
  - **Step 2: Hashing Input** - The SHA-1 hash must be calculated over the concatenated bytes of the Header + Content.
  - **Step 3: Compression** - Use the flate2 crate to compress the Header + Content bytes using Zlib.
  - **Step 4: Storage Path** - Use the calculated 40-character hex hash. The path must be: `.git/objects/<first two characters of hash>/<remaining 38 characters>`.
- **Validation Requirement:** The final hash must match the output of `git hash-object -w --stdin` when fed the same data.

#### Public fn read_object
- **Signature:** read_object(hash: &str) -> Result<Object, CrustError>
- **Purpose:** The inverse of store_object. Reads a hash, decompresses the data, and extracts the content.
- **Steps:**
  - **Step 1: Retrieval** - Read the compressed object file from the derived path (e.g., `.git/objects/ab/123...`). Handle `ObjectNotFound` error if the file doesn't exist.
  - **Step 2: Decompression** - Use the flate2 crate to decompress the file contents using Zlib.
  - **Step 3: Header Parsing** - Find the first null byte (0u8) in the decompressed byte stream. This separates the header (type/size) from the content.
  - **Step 4: Output** - Return a populated `Object` struct containing the extracted `ObjectType` and the raw `Vec<u8>` content.
- **Validation Requirement:** Ensure the header parsing correctly handles `ObjectType::Commit` and `ObjectType::Tree` (text formats) as well as `ObjectType::Blob` (potentially binary).

### Tree Objects (src/plumbing/trees.rs)
**Goal:** Implement Merkle Tree logic to capture directory structure.

#### Tree Model
- Define a public struct 'TreeEntry' { mode: String, name: String, hash: [u8; 20] }.
- Note: The hash must be stored as raw 20 bytes ([u8; 20]), not a hex string.

#### Public fn make_snapshot
- **Signature:** make_snapshot(path: &Path) -> Result<String, CrustError>
- **Purpose:** Recursively walk the directory, serialize the structure, and store it as a Tree object (the Merkle Tree node).
- **Steps:**
  - **Step 1: Directory Traversal** - Use `std::fs` to recursively traverse the directory. Ignore the `.git` directory.
  - **Step 2: Entry Handling** - For files: Call `store_object` to get the Blob hash. For directories: Recursively call `make_snapshot` to get the Tree hash.
  - **Step 3: Tree Entry Format** - Each entry in the resulting Tree object's content must be: `<mode> <name>\0<20 raw byte hash>`. The hash must be the 20 raw bytes, NOT the 40-character hex string.
  - **Step 4: Sorting** - Tree entries must be sorted lexicographically by name before serialization. This is critical for matching Git's hash.
  - **Step 5: Storage** - Concatenate all serialized entries, call `store_object` with `ObjectType::Tree`.
- **Validation Requirement:** The final Tree hash must exactly match the hash produced by official `git write-tree` for the same directory state.

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
