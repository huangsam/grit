use std::path::Path;
use crate::error::CrustError;
use sha1::{Digest, Sha1};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use std::fs;

/// Represents the type of Git object stored in the repository.
/// Git objects come in three fundamental types that form the basis of the version control system.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    /// A blob object contains the content of a file. It stores raw file data without any
    /// metadata about the file itself (like filename or permissions).
    Blob,

    /// A tree object represents a directory structure. It contains references to blobs
    /// (for files) and other trees (for subdirectories), along with their names and permissions.
    Tree,

    /// A commit object represents a snapshot of the repository at a point in time.
    /// It contains metadata about the commit (author, message, timestamp) and references
    /// to the tree object representing the file structure and parent commits.
    Commit,
}

/// Represents a Git object with its type and content.
/// This is the core data structure for storing and retrieving objects from the repository.
/// Objects are immutable once created - they are identified by their content hash.
#[derive(Debug, Clone)]
pub struct Object {
    /// The type of this object (Blob, Tree, or Commit)
    pub obj_type: ObjectType,

    /// The raw content of the object. For blobs, this is file data.
    /// For trees and commits, this is structured text data.
    pub content: Vec<u8>,
}

/// Stores an object in the Git object database using content-addressable storage.
/// This is the core function that implements Git's CAS mechanism.
///
/// The process involves:
/// 1. Constructing a Git header with object type and size
/// 2. Computing SHA-1 hash of header + content
/// 3. Compressing the data with zlib
/// 4. Storing in .crust/objects/xx/yyyy... where xx are first 2 hex chars of hash
///
/// # Arguments
/// * `content` - The raw bytes to store in the object
/// * `obj_type` - The type of object being stored
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex SHA-1 hash of the stored object
/// * `Err(CrustError)` - If storage fails due to I/O errors or other issues
///
/// # Validation
/// The returned hash should match `git hash-object -w --stdin` for the same input.
pub fn store_object(content: &[u8], obj_type: ObjectType) -> Result<String, CrustError> {
    // Step 1: Header Construction
    let type_str = match obj_type {
        ObjectType::Blob => "blob",
        ObjectType::Tree => "tree",
        ObjectType::Commit => "commit",
    };
    let header = format!("{} {}\0", type_str, content.len());
    let header_bytes = header.as_bytes();

    // Step 2: Hashing Input
    let mut hasher = Sha1::new();
    hasher.update(header_bytes);
    hasher.update(content);
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);

    // Step 3: Compression
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(header_bytes)?;
    encoder.write_all(content)?;
    let compressed_data = encoder.finish()?;

    // Step 4: Storage Path
    let (prefix, suffix) = hash_hex.split_at(2);
    let object_dir = Path::new(".crust").join("objects").join(prefix);
    fs::create_dir_all(&object_dir)?;

    let object_path = object_dir.join(suffix);
    fs::write(object_path, compressed_data)?;

    Ok(hash_hex)
}

/// Retrieves an object from the Git object database by its hash.
/// This is the inverse operation of store_object.
///
/// The process involves:
/// 1. Constructing the object path from the hash
/// 2. Reading and decompressing the stored data
/// 3. Parsing the Git header to extract type and content
/// 4. Returning the reconstructed Object
///
/// # Arguments
/// * `hash` - The 40-character hex SHA-1 hash of the object to retrieve
///
/// # Returns
/// * `Ok(Object)` - The retrieved object with its type and content
/// * `Err(CrustError)` - If the object doesn't exist or is corrupted
///
/// # Errors
/// * `ObjectNotFound` - If no object with the given hash exists
/// * `CorruptObject` - If the stored data is malformed or decompression fails
pub fn read_object(hash: &str) -> Result<Object, CrustError> {
    // TODO: Implement
    Ok(Object {
        obj_type: ObjectType::Blob,
        content: Vec::new(),
    })
}
