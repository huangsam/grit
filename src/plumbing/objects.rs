use std::path::Path;
use crate::error::GritError;
use sha1::{Digest, Sha1};
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use std::io::{Read, Write, BufWriter, BufReader};
use std::fs;
use crate::cache;

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
/// 4. Storing in .grit/objects/xx/yyyy... where xx are first 2 hex chars of hash
///
/// # Arguments
/// * `content` - The raw bytes to store in the object
/// * `obj_type` - The type of object being stored
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(hash)` - The 40-character hex SHA-1 hash of the stored object
/// * `Err(GritError)` - If storage fails due to I/O errors or other issues
///
/// # Validation
/// The returned hash should match `git hash-object -w --stdin` for the same input.
pub fn store_object(content: &[u8], obj_type: ObjectType, repo_root: &Path) -> Result<String, GritError> {
    // Check hash cache first - compute content hash to see if we've stored this before
    let obj_type_u8 = obj_type.clone() as u8;
    let content_hash = format!("{}_{}", obj_type_u8, hex::encode(Sha1::digest(content)));

    if let Some(cached_hash) = cache::GLOBAL_CACHE.hash_cache.get(&content_hash) {
        // Verify the object still exists on disk
        let (prefix, suffix) = cached_hash.split_at(2);
        let object_path = repo_root.join(".grit").join("objects").join(prefix).join(suffix);
        if object_path.exists() {
            return Ok(cached_hash);
        }
        // Object was evicted from disk, fall through to recompute
    }

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

    // Cache the computed hash
    cache::GLOBAL_CACHE.hash_cache.put(content_hash, hash_hex.clone());

    // Step 3: Compression
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(header_bytes)?;
    encoder.write_all(content)?;
    let compressed_data = encoder.finish()?;

    // Step 4: Storage Path
    let (prefix, suffix) = hash_hex.split_at(2);
    let object_dir = repo_root.join(".grit").join("objects").join(prefix);
    fs::create_dir_all(&object_dir)?;

    let object_path = object_dir.join(suffix);
    let file = fs::File::create(object_path)?;
    let mut buf_writer = BufWriter::new(file);
    buf_writer.write_all(&compressed_data)?;
    buf_writer.flush()?;

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
/// * `repo_root` - The root directory of the repository
///
/// # Returns
/// * `Ok(Object)` - The retrieved object with its type and content
/// * `Err(GritError)` - If the object doesn't exist or is corrupted
///
/// # Errors
/// * `ObjectNotFound` - If no object with the given hash exists
/// * `CorruptObject` - If the stored data is malformed or decompression fails
pub fn read_object(hash: &str, repo_root: &Path) -> Result<Object, GritError> {
    // Check cache first
    if let Some(cached_object) = cache::GLOBAL_CACHE.object_cache.get(hash) {
        return Ok(cached_object);
    }

    // Step 1: Retrieval
    let (prefix, suffix) = hash.split_at(2);
    let object_path = repo_root.join(".grit").join("objects").join(prefix).join(suffix);

    if !object_path.exists() {
        return Err(GritError::ObjectNotFound(hash.to_string()));
    }

    let file = fs::File::open(object_path)?;
    let mut buf_reader = BufReader::new(file);
    let mut compressed_data = Vec::new();
    buf_reader.read_to_end(&mut compressed_data)?;

    // Step 2: Decompression
    let mut decoder = ZlibDecoder::new(&compressed_data[..]);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;

    // Step 3: Header Parsing
    let null_pos = decompressed_data.iter().position(|&b| b == 0)
        .ok_or_else(|| GritError::CorruptObject("No null byte found in object data".to_string()))?;

    let header = &decompressed_data[..null_pos];
    let content = &decompressed_data[null_pos + 1..];

    // Parse header: "type size"
    let header_str = std::str::from_utf8(header)
        .map_err(|_| GritError::CorruptObject("Invalid UTF-8 in object header".to_string()))?;

    let mut parts = header_str.split_whitespace();
    let type_str = parts.next()
        .ok_or_else(|| GritError::CorruptObject("Missing type in object header".to_string()))?;

    let obj_type = match type_str {
        "blob" => ObjectType::Blob,
        "tree" => ObjectType::Tree,
        "commit" => ObjectType::Commit,
        _ => return Err(GritError::CorruptObject(format!("Unknown object type: {}", type_str))),
    };

    // Step 4: Output
    let object = Object {
        obj_type,
        content: content.to_vec(),
    };

    // Cache the object for future use
    cache::GLOBAL_CACHE.object_cache.put(hash.to_string(), object.clone());

    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::repository::initialize_repo;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Initialize repository
        initialize_repo(temp_dir.path()).unwrap();

        temp_dir
    }

    #[test]
    fn test_store_and_read_blob() {
        let test_dir = setup_test_repo();

        let content = b"Hello, World!";
        let hash = store_object(content, ObjectType::Blob, test_dir.path()).unwrap();

        // Verify hash is 40 characters
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Read back the object
        let object = read_object(&hash, test_dir.path()).unwrap();
        assert_eq!(object.obj_type, ObjectType::Blob);
        assert_eq!(object.content, content);
    }

    #[test]
    fn test_store_and_read_tree() {
        let test_dir = setup_test_repo();

        let tree_content = b"100644 file.txt\x00\xaa\xbb\xcc\xdd\xee\xff\x00\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc";
        let hash = store_object(tree_content, ObjectType::Tree, test_dir.path()).unwrap();

        // Read back the object
        let object = read_object(&hash, test_dir.path()).unwrap();
        assert_eq!(object.obj_type, ObjectType::Tree);
        assert_eq!(object.content, tree_content);
    }

    #[test]
    fn test_store_and_read_commit() {
        let test_dir = setup_test_repo();

        let commit_content = b"tree abc123\nauthor Test <test@example.com> 1234567890 +0000\n\nTest commit";
        let hash = store_object(commit_content, ObjectType::Commit, test_dir.path()).unwrap();

        // Read back the object
        let object = read_object(&hash, test_dir.path()).unwrap();
        assert_eq!(object.obj_type, ObjectType::Commit);
        assert_eq!(object.content, commit_content);
    }

    #[test]
    fn test_read_nonexistent_object() {
        let test_dir = setup_test_repo();

        let result = read_object("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", test_dir.path());
        assert!(result.is_err());

        if let Err(GritError::ObjectNotFound(hash)) = result {
            assert_eq!(hash, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        } else {
            panic!("Expected ObjectNotFound error");
        }
    }

    #[test]
    fn test_store_empty_content() {
        let test_dir = setup_test_repo();

        let content = b"";
        let hash = store_object(content, ObjectType::Blob, test_dir.path()).unwrap();

        // Read back the object
        let object = read_object(&hash, test_dir.path()).unwrap();
        assert_eq!(object.obj_type, ObjectType::Blob);
        assert_eq!(object.content, content);
    }

    // Property-based tests
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_store_read_blob_roundtrip(content in prop::collection::vec(any::<u8>(), 0..10000)) {
            let test_dir = setup_test_repo();

            // Store the content
            let hash = store_object(&content, ObjectType::Blob, test_dir.path())?;

            // Read it back
            let read_obj = read_object(&hash, test_dir.path())?;

            // Verify roundtrip
            prop_assert_eq!(read_obj.obj_type, ObjectType::Blob);
            prop_assert_eq!(read_obj.content, content);
        }

        #[test]
        fn test_store_read_tree_roundtrip(
            entries in prop::collection::vec(
                (prop::string::string_regex("[a-zA-Z0-9_.-]{1,10}").unwrap()
                    .prop_filter("Exclude problematic filenames", |s| s != "." && s != ".." && !s.contains('/') && !s.starts_with('.') && s != ".grit" && s != "target"),
                 prop::collection::vec(any::<u8>(), 0..100)),
                1..5
            ).prop_map(|mut entries| {
                // Ensure unique filenames by appending index to duplicates
                let mut seen = std::collections::HashSet::new();
                for (i, (filename, _)) in entries.iter_mut().enumerate() {
                    if !seen.insert(filename.clone()) {
                        *filename = format!("{}_{}", filename, i);
                    }
                }
                entries
            })
        ) {
            let test_dir = setup_test_repo();

            // Create files for the tree
            for (filename, content) in &entries {
                std::fs::write(test_dir.path().join(filename), content)?;
            }

            // Create tree snapshot
            let tree_hash = crate::plumbing::trees::make_snapshot(test_dir.path(), test_dir.path())?;

            // Read tree object
            let tree_obj = read_object(&tree_hash, test_dir.path())?;
            prop_assert_eq!(tree_obj.obj_type, ObjectType::Tree);

            // Parse tree entries
            let parsed_entries = crate::plumbing::checkout::parse_tree_entries(&tree_obj.content)?;
            prop_assert_eq!(parsed_entries.len(), entries.len());
        }
    }

    #[test]
    fn test_store_large_file() {
        let test_dir = setup_test_repo();

        // Create a 10MB file
        let large_content = vec![42u8; 10 * 1024 * 1024];
        let hash = store_object(&large_content, ObjectType::Blob, test_dir.path()).unwrap();

        // Verify it can be read back
        let read_obj = read_object(&hash, test_dir.path()).unwrap();
        assert_eq!(read_obj.obj_type, ObjectType::Blob);
        assert_eq!(read_obj.content.len(), large_content.len());
        assert_eq!(read_obj.content, large_content);
    }

    #[test]
    fn test_performance_buffered_io() {
        let test_dir = setup_test_repo();

        // Test with various file sizes
        let sizes = [1024, 10 * 1024, 100 * 1024, 1024 * 1024]; // 1KB, 10KB, 100KB, 1MB

        for &size in &sizes {
            let content = vec![b'A'; size];

            // Time the store operation
            let start = std::time::Instant::now();
            let hash = store_object(&content, ObjectType::Blob, test_dir.path()).unwrap();
            let store_time = start.elapsed();

            // Time the read operation
            let start = std::time::Instant::now();
            let read_obj = read_object(&hash, test_dir.path()).unwrap();
            let read_time = start.elapsed();

            // Verify correctness
            assert_eq!(read_obj.obj_type, ObjectType::Blob);
            assert_eq!(read_obj.content.len(), size);

            println!("Size: {}KB - Store: {:.2}ms, Read: {:.2}ms",
                    size / 1024,
                    store_time.as_secs_f64() * 1000.0,
                    read_time.as_secs_f64() * 1000.0);
        }
    }

    #[test]
    fn test_unicode_filenames() {
        let test_dir = setup_test_repo();

        // Create files with Unicode names
        let unicode_files = vec![
            ("🚀_rocket.txt", "Space content"),
            ("café.txt", "Coffee content"),
            ("тест.txt", "Test content"),
            ("文件.txt", "File content"),
        ];

        for (filename, content) in &unicode_files {
            std::fs::write(test_dir.path().join(filename), content).unwrap();
        }

        // Create tree snapshot
        let tree_hash = crate::plumbing::trees::make_snapshot(test_dir.path(), test_dir.path()).unwrap();

        // Read and verify tree contains all files
        let tree_obj = read_object(&tree_hash, test_dir.path()).unwrap();
        assert_eq!(tree_obj.obj_type, ObjectType::Tree);

        let entries = crate::plumbing::checkout::parse_tree_entries(&tree_obj.content).unwrap();
        assert_eq!(entries.len(), unicode_files.len());

        // Verify filenames are preserved
        let entry_names: std::collections::HashSet<_> = entries.iter().map(|e| e.name.as_str()).collect();
        let expected_names: std::collections::HashSet<_> = unicode_files.iter().map(|(name, _)| *name).collect();
        assert_eq!(entry_names, expected_names);
    }

    #[test]
    fn test_corrupted_object_handling() {
        let test_dir = setup_test_repo();

        // Create a valid object first
        let content = b"Hello World";
        let hash = store_object(content, ObjectType::Blob, test_dir.path()).unwrap();

        // Manually corrupt the object file
        let object_path = test_dir.path().join(".grit").join("objects")
            .join(&hash[..2]).join(&hash[2..]);

        let mut corrupted_content = std::fs::read(&object_path).unwrap();
        if corrupted_content.len() > 10 {
            corrupted_content[10] ^= 0xFF; // Flip some bits
        }
        std::fs::write(&object_path, corrupted_content).unwrap();

        // Clear cache to ensure we read from disk
        cache::GLOBAL_CACHE.object_cache.clear();

        // Attempt to read should fail gracefully
        let result = read_object(&hash, test_dir.path());
        assert!(result.is_err());

        // Should be an Io error due to corrupted compression
        match result {
            Err(crate::error::GritError::Io(_)) => {},
            Err(e) => panic!("Expected Io error, got: {:?}", e),
            _ => panic!("Expected Io error"),
        }
    }
}
