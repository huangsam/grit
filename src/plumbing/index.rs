//! Git Index Implementation
//!
//! The Git index (staging area) is a binary file that tracks the state of files
//! that have been staged for the next commit. This module implements Git's
//! index format for reading and writing index files.
//!
//! ## Index Format Overview
//!
//! The Git index has the following structure:
//! - Header: "DIRC" signature, version (2 or 3), entry count
//! - Entries: Sorted array of index entries with file metadata
//! - Extensions: Optional extensions for additional data
//! - Trailer: SHA-1 hash of the entire index content
//!
//! ## Index Entry Structure
//!
//! Each entry contains:
//! - Timestamps (ctime, mtime) with nanosecond precision
//! - Device/inode numbers for hard link detection
//! - File mode (permissions)
//! - User/group IDs
//! - File size
//! - SHA-1 hash of the staged content
//! - Flags (stage, name length, etc.)
//! - Null-terminated path name

use crate::error::GritError;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Git index header signature
const INDEX_SIGNATURE: &[u8; 4] = b"DIRC";

/// Current Git index version
const INDEX_VERSION: u32 = 2;

/// Index entry representing a staged file
#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    /// Creation time (seconds since Unix epoch)
    pub ctime_sec: u32,
    /// Creation time (nanoseconds)
    pub ctime_nsec: u32,
    /// Modification time (seconds since Unix epoch)
    pub mtime_sec: u32,
    /// Modification time (nanoseconds)
    pub mtime_nsec: u32,
    /// Device number
    pub dev: u32,
    /// Inode number
    pub ino: u32,
    /// File mode (permissions)
    pub mode: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// File size
    pub size: u32,
    /// SHA-1 hash of the staged content
    pub hash: [u8; 20],
    /// Flags (stage, name length, etc.)
    pub flags: u16,
    /// File path (null-terminated)
    pub path: String,
}

/// Git index containing all staged entries
#[derive(Debug, Clone)]
pub struct Index {
    /// Index entries sorted by path
    pub entries: Vec<IndexEntry>,
    /// Index version (2 or 3)
    pub version: u32,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    /// Create a new empty index
    pub fn new() -> Self {
        Index {
            entries: Vec::new(),
            version: INDEX_VERSION,
        }
    }

    /// Add or update an entry in the index
    pub fn add_entry(&mut self, entry: IndexEntry) {
        // Remove existing entry with same path if it exists
        self.entries.retain(|e| e.path != entry.path);
        // Insert in sorted order
        let pos = self.entries.partition_point(|e| e.path < entry.path);
        self.entries.insert(pos, entry);
    }
}

/// Create an index entry from file metadata and content hash
pub fn create_index_entry(
    path: &Path,
    hash: &[u8; 20],
    repo_root: &Path,
) -> Result<IndexEntry, GritError> {
    let metadata = fs::metadata(path)?;
    let ctime = metadata.created()?;
    let mtime = metadata.modified()?;

    // Convert times to Unix timestamps with nanoseconds
    let ctime_duration = ctime.duration_since(UNIX_EPOCH)?;
    let mtime_duration = mtime.duration_since(UNIX_EPOCH)?;

    // Get device and inode (Unix-specific, use defaults for other platforms)
    #[cfg(unix)]
    let (dev, ino) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev() as u32, metadata.ino() as u32)
    };
    #[cfg(not(unix))]
    let (dev, ino) = (0, 0);

    // Get user/group IDs (Unix-specific)
    #[cfg(unix)]
    let (uid, gid) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.uid(), metadata.gid())
    };
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);

    let mode = if cfg!(unix) {
        metadata.permissions().mode()
    } else {
        // Default mode for non-Unix systems
        0o100644
    };
    let size = metadata.len() as u32;

    // Calculate path relative to repository root
    let relative_path = path
        .strip_prefix(repo_root)
        .map_err(|_| GritError::RepositoryError("File not in repository".to_string()))?
        .to_string_lossy()
        .to_string();

    Ok(IndexEntry {
        ctime_sec: ctime_duration.as_secs() as u32,
        ctime_nsec: ctime_duration.subsec_nanos(),
        mtime_sec: mtime_duration.as_secs() as u32,
        mtime_nsec: mtime_duration.subsec_nanos(),
        dev,
        ino,
        mode,
        uid,
        gid,
        size,
        hash: *hash,
        flags: 0, // No special flags for now
        path: relative_path,
    })
}

/// Read the Git index from disk
pub fn read_index(repo_root: &Path) -> Result<Index, GritError> {
    let index_path = repo_root.join(".grit").join("index");

    if !index_path.exists() {
        return Ok(Index::new());
    }

    let mut reader = BufReader::new(fs::File::open(index_path)?);

    // Read header
    let mut signature = [0u8; 4];
    reader.read_exact(&mut signature)?;
    if &signature != INDEX_SIGNATURE {
        return Err(GritError::RepositoryError(
            "Invalid index signature".to_string(),
        ));
    }

    let mut version_bytes = [0u8; 4];
    reader.read_exact(&mut version_bytes)?;
    let version = u32::from_be_bytes(version_bytes);

    let mut entry_count_bytes = [0u8; 4];
    reader.read_exact(&mut entry_count_bytes)?;
    let entry_count = u32::from_be_bytes(entry_count_bytes) as usize;

    // Read entries
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let entry = read_index_entry(&mut reader)?;
        entries.push(entry);
    }

    // TODO: Read extensions if present
    // TODO: Verify hash

    Ok(Index { entries, version })
}

/// Write the Git index to disk
pub fn write_index(index: &Index, repo_root: &Path) -> Result<(), GritError> {
    let index_path = repo_root.join(".grit").join("index");
    let mut writer = BufWriter::new(fs::File::create(index_path)?);

    // Write header
    writer.write_all(INDEX_SIGNATURE)?;
    writer.write_all(&index.version.to_be_bytes())?;
    writer.write_all(&(index.entries.len() as u32).to_be_bytes())?;

    // Write entries
    for entry in &index.entries {
        write_index_entry(&mut writer, entry)?;
    }

    // TODO: Write extensions if needed
    // TODO: Write hash

    writer.flush()?;
    Ok(())
}

/// Read a single index entry from the reader
fn read_index_entry<R: Read>(reader: &mut R) -> Result<IndexEntry, GritError> {
    let mut ctime_sec_bytes = [0u8; 4];
    reader.read_exact(&mut ctime_sec_bytes)?;
    let ctime_sec = u32::from_be_bytes(ctime_sec_bytes);

    let mut ctime_nsec_bytes = [0u8; 4];
    reader.read_exact(&mut ctime_nsec_bytes)?;
    let ctime_nsec = u32::from_be_bytes(ctime_nsec_bytes);

    let mut mtime_sec_bytes = [0u8; 4];
    reader.read_exact(&mut mtime_sec_bytes)?;
    let mtime_sec = u32::from_be_bytes(mtime_sec_bytes);

    let mut mtime_nsec_bytes = [0u8; 4];
    reader.read_exact(&mut mtime_nsec_bytes)?;
    let mtime_nsec = u32::from_be_bytes(mtime_nsec_bytes);

    let mut dev_bytes = [0u8; 4];
    reader.read_exact(&mut dev_bytes)?;
    let dev = u32::from_be_bytes(dev_bytes);

    let mut ino_bytes = [0u8; 4];
    reader.read_exact(&mut ino_bytes)?;
    let ino = u32::from_be_bytes(ino_bytes);

    let mut mode_bytes = [0u8; 4];
    reader.read_exact(&mut mode_bytes)?;
    let mode = u32::from_be_bytes(mode_bytes);

    let mut uid_bytes = [0u8; 4];
    reader.read_exact(&mut uid_bytes)?;
    let uid = u32::from_be_bytes(uid_bytes);

    let mut gid_bytes = [0u8; 4];
    reader.read_exact(&mut gid_bytes)?;
    let gid = u32::from_be_bytes(gid_bytes);

    let mut size_bytes = [0u8; 4];
    reader.read_exact(&mut size_bytes)?;
    let size = u32::from_be_bytes(size_bytes);

    let mut hash = [0u8; 20];
    reader.read_exact(&mut hash)?;

    let mut flags_bytes = [0u8; 2];
    reader.read_exact(&mut flags_bytes)?;
    let flags = u16::from_be_bytes(flags_bytes);

    // Read path (null-terminated)
    let mut path_bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        reader.read_exact(&mut byte)?;
        if byte[0] == 0 {
            break;
        }
        path_bytes.push(byte[0]);
    }

    let path = String::from_utf8(path_bytes)
        .map_err(|_| GritError::RepositoryError("Invalid UTF-8 in index path".to_string()))?;

    // Skip padding to align to 8-byte boundary
    let entry_size = 62 + path.len() + 1; // 62 bytes fixed + path + null
    let padding = (8 - (entry_size % 8)) % 8;
    for _ in 0..padding {
        reader.read_exact(&mut byte)?;
        if byte[0] != 0 {
            return Err(GritError::RepositoryError(
                "Non-zero padding in index entry".to_string(),
            ));
        }
    }

    Ok(IndexEntry {
        ctime_sec,
        ctime_nsec,
        mtime_sec,
        mtime_nsec,
        dev,
        ino,
        mode,
        uid,
        gid,
        size,
        hash,
        flags,
        path,
    })
}

/// Write a single index entry to the writer
fn write_index_entry<W: Write>(writer: &mut W, entry: &IndexEntry) -> Result<(), GritError> {
    writer.write_all(&entry.ctime_sec.to_be_bytes())?;
    writer.write_all(&entry.ctime_nsec.to_be_bytes())?;
    writer.write_all(&entry.mtime_sec.to_be_bytes())?;
    writer.write_all(&entry.mtime_nsec.to_be_bytes())?;
    writer.write_all(&entry.dev.to_be_bytes())?;
    writer.write_all(&entry.ino.to_be_bytes())?;
    writer.write_all(&entry.mode.to_be_bytes())?;
    writer.write_all(&entry.uid.to_be_bytes())?;
    writer.write_all(&entry.gid.to_be_bytes())?;
    writer.write_all(&entry.size.to_be_bytes())?;
    writer.write_all(&entry.hash)?;
    writer.write_all(&entry.flags.to_be_bytes())?;

    // Write path as null-terminated string
    writer.write_all(entry.path.as_bytes())?;
    writer.write_all(&[0])?;

    // Add padding to align to 8-byte boundary
    let entry_size = 62 + entry.path.len() + 1; // 62 bytes fixed + path + null
    let padding = (8 - (entry_size % 8)) % 8;
    for _ in 0..padding {
        writer.write_all(&[0])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_new() {
        let index = Index::new();
        assert!(index.entries.is_empty());
        assert_eq!(index.entries.len(), 0);
        assert_eq!(index.version, INDEX_VERSION);
    }

    #[test]
    fn test_index_add_entry() {
        let mut index = Index::new();

        let entry1 = IndexEntry {
            ctime_sec: 1234567890,
            ctime_nsec: 0,
            mtime_sec: 1234567890,
            mtime_nsec: 0,
            dev: 1,
            ino: 123,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 42,
            hash: [0u8; 20],
            flags: 0,
            path: "file1.txt".to_string(),
        };

        let entry2 = IndexEntry {
            ctime_sec: 1234567891,
            ctime_nsec: 0,
            mtime_sec: 1234567891,
            mtime_nsec: 0,
            dev: 1,
            ino: 124,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 24,
            hash: [1u8; 20],
            flags: 0,
            path: "file2.txt".to_string(),
        };

        index.add_entry(entry1.clone());
        index.add_entry(entry2.clone());

        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].path, "file1.txt");
        assert_eq!(index.entries[1].path, "file2.txt");
    }

    #[test]
    fn test_index_update_entry() {
        let mut index = Index::new();

        let entry1 = IndexEntry {
            ctime_sec: 1234567890,
            ctime_nsec: 0,
            mtime_sec: 1234567890,
            mtime_nsec: 0,
            dev: 1,
            ino: 123,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 42,
            hash: [0u8; 20],
            flags: 0,
            path: "file.txt".to_string(),
        };

        let entry2 = IndexEntry {
            ctime_sec: 1234567891,
            ctime_nsec: 0,
            mtime_sec: 1234567891,
            mtime_nsec: 0,
            dev: 1,
            ino: 124,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 24,
            hash: [1u8; 20],
            flags: 0,
            path: "file.txt".to_string(),
        };

        index.add_entry(entry1);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].size, 42);

        index.add_entry(entry2);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].size, 24);
    }

    #[test]
    fn test_index_remove_entry() {
        let mut index = Index::new();

        let entry = IndexEntry {
            ctime_sec: 1234567890,
            ctime_nsec: 0,
            mtime_sec: 1234567890,
            mtime_nsec: 0,
            dev: 1,
            ino: 123,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 42,
            hash: [0u8; 20],
            flags: 0,
            path: "file.txt".to_string(),
        };

        index.add_entry(entry);
        assert_eq!(index.entries.len(), 1);

        index.entries.retain(|e| e.path != "file.txt");
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_index_get_entry() {
        let mut index = Index::new();

        let entry = IndexEntry {
            ctime_sec: 1234567890,
            ctime_nsec: 0,
            mtime_sec: 1234567890,
            mtime_nsec: 0,
            dev: 1,
            ino: 123,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 42,
            hash: [0u8; 20],
            flags: 0,
            path: "file.txt".to_string(),
        };

        index.add_entry(entry.clone());

        let retrieved = index.entries.iter().find(|e| e.path == "file.txt");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().size, 42);

        let not_found = index.entries.iter().find(|e| e.path == "nonexistent.txt");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_create_index_entry() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        let hash = [1u8; 20];
        let entry = create_index_entry(&file_path, &hash, temp_dir.path()).unwrap();

        assert_eq!(entry.path, "test.txt");
        assert_eq!(entry.hash, hash);
        assert_eq!(entry.size, 12); // "test content" is 12 bytes
        assert!(entry.ctime_sec > 0);
        assert!(entry.mtime_sec > 0);
    }

    #[test]
    fn test_read_write_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();

        // Create .grit directory
        fs::create_dir(repo_root.join(".grit")).unwrap();

        // Create an index with some entries
        let mut index = Index::new();
        let hash1 = [1u8; 20];
        let hash2 = [2u8; 20];

        let entry1 = IndexEntry {
            ctime_sec: 1234567890,
            ctime_nsec: 123456789,
            mtime_sec: 1234567890,
            mtime_nsec: 123456789,
            dev: 1,
            ino: 12345,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 100,
            hash: hash1,
            flags: 0,
            path: "file1.txt".to_string(),
        };

        let entry2 = IndexEntry {
            ctime_sec: 1234567891,
            ctime_nsec: 123456790,
            mtime_sec: 1234567891,
            mtime_nsec: 123456790,
            dev: 1,
            ino: 12346,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            size: 200,
            hash: hash2,
            flags: 0,
            path: "file2.txt".to_string(),
        };

        index.add_entry(entry1.clone());
        index.add_entry(entry2.clone());

        // Write the index
        write_index(&index, repo_root).unwrap();

        // Read it back
        let read_index = read_index(repo_root).unwrap();

        assert_eq!(read_index.entries.len(), 2);
        assert_eq!(read_index.entries[0].path, "file1.txt");
        assert_eq!(read_index.entries[1].path, "file2.txt");
        assert_eq!(read_index.entries[0].hash, hash1);
        assert_eq!(read_index.entries[1].hash, hash2);
    }

    #[test]
    fn test_read_empty_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();

        // Reading a non-existent index should return an empty index
        let index = read_index(repo_root).unwrap();
        assert_eq!(index.entries.len(), 0);
    }
}
