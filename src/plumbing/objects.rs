use std::path::Path;
use crate::error::CrustError;
use sha1::{Digest, Sha1};
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use std::io::{Read, Write};
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
    // Step 1: Retrieval
    let (prefix, suffix) = hash.split_at(2);
    let object_path = Path::new(".crust").join("objects").join(prefix).join(suffix);

    if !object_path.exists() {
        return Err(CrustError::ObjectNotFound(hash.to_string()));
    }

    let compressed_data = fs::read(object_path)?;

    // Step 2: Decompression
    let mut decoder = ZlibDecoder::new(&compressed_data[..]);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;

    // Step 3: Header Parsing
    let null_pos = decompressed_data.iter().position(|&b| b == 0)
        .ok_or_else(|| CrustError::CorruptObject("No null byte found in object data".to_string()))?;

    let header = &decompressed_data[..null_pos];
    let content = &decompressed_data[null_pos + 1..];

    // Parse header: "type size"
    let header_str = std::str::from_utf8(header)
        .map_err(|_| CrustError::CorruptObject("Invalid UTF-8 in object header".to_string()))?;

    let mut parts = header_str.split_whitespace();
    let type_str = parts.next()
        .ok_or_else(|| CrustError::CorruptObject("Missing type in object header".to_string()))?;

    let obj_type = match type_str {
        "blob" => ObjectType::Blob,
        "tree" => ObjectType::Tree,
        "commit" => ObjectType::Commit,
        _ => return Err(CrustError::CorruptObject(format!("Unknown object type: {}", type_str))),
    };

    // Step 4: Output
    Ok(Object {
        obj_type,
        content: content.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use crate::repository::initialize_repo;

    fn setup_test_repo() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("crust_test_objects_{}", timestamp));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Initialize repository
        initialize_repo().unwrap();

        temp_dir
    }

    fn cleanup_test_repo(test_dir: PathBuf) {
        if test_dir.exists() {
            env::set_current_dir(env::temp_dir().parent().unwrap()).unwrap();
            fs::remove_dir_all(test_dir).unwrap();
        }
    }

    #[test]
    fn test_store_and_read_blob() {
        let test_dir = setup_test_repo();

        let content = b"Hello, World!";
        let hash = store_object(content, ObjectType::Blob).unwrap();

        // Verify hash is 40 characters
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Read back the object
        let object = read_object(&hash).unwrap();
        assert_eq!(object.obj_type, ObjectType::Blob);
        assert_eq!(object.content, content);

        cleanup_test_repo(test_dir);
    }

    #[test]
    fn test_store_and_read_tree() {
        let test_dir = setup_test_repo();

        let tree_content = b"100644 file.txt\x00\xaa\xbb\xcc\xdd\xee\xff\x00\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc";
        let hash = store_object(tree_content, ObjectType::Tree).unwrap();

        // Read back the object
        let object = read_object(&hash).unwrap();
        assert_eq!(object.obj_type, ObjectType::Tree);
        assert_eq!(object.content, tree_content);

        cleanup_test_repo(test_dir);
    }

    #[test]
    fn test_store_and_read_commit() {
        let test_dir = setup_test_repo();

        let commit_content = b"tree abc123\nauthor Test <test@example.com> 1234567890 +0000\n\nTest commit";
        let hash = store_object(commit_content, ObjectType::Commit).unwrap();

        // Read back the object
        let object = read_object(&hash).unwrap();
        assert_eq!(object.obj_type, ObjectType::Commit);
        assert_eq!(object.content, commit_content);

        cleanup_test_repo(test_dir);
    }

    #[test]
    fn test_read_nonexistent_object() {
        let test_dir = setup_test_repo();

        let result = read_object("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(result.is_err());

        if let Err(CrustError::ObjectNotFound(hash)) = result {
            assert_eq!(hash, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        } else {
            panic!("Expected ObjectNotFound error");
        }

        cleanup_test_repo(test_dir);
    }

    #[test]
    fn test_store_empty_content() {
        let test_dir = setup_test_repo();

        let content = b"";
        let hash = store_object(content, ObjectType::Blob).unwrap();

        // Read back the object
        let object = read_object(&hash).unwrap();
        assert_eq!(object.obj_type, ObjectType::Blob);
        assert_eq!(object.content, content);

        cleanup_test_repo(test_dir);
    }
}
