//! Error handling and reporting
//!
//! Comprehensive error types and handling for all Grit operations.
//! Provides detailed error information while maintaining clean error propagation
//! throughout the codebase.

use std::error::Error;
use std::fmt;

/// Represents all possible errors that can occur in the Grit Git plumbing implementation.
/// This enum centralizes error handling to provide consistent error reporting and
/// easy conversion from standard library errors.
#[derive(Debug)]
pub enum GritError {
    /// An I/O error occurred during file operations (reading, writing, creating directories).
    /// This wraps std::io::Error for seamless error propagation.
    Io(std::io::Error),

    /// The requested object could not be found in the repository.
    /// Contains the hash of the missing object for debugging.
    ObjectNotFound(String),

    /// An object was found but its content is corrupted or malformed.
    /// Contains a descriptive message about what was wrong with the object.
    CorruptObject(String),

    /// Structured repository-related error. Prefer using `RepoError` variants to
    /// allow callers to match specific cases (e.g., `NoCommits`).
    RepositoryError(RepoError),

    /// Error occurred while decoding hex strings.
    /// This wraps hex::FromHexError for hash decoding operations.
    HexDecode(hex::FromHexError),

    /// Error occurred while working with system time.
    /// This wraps std::time::SystemTimeError for timestamp operations.
    TimeError(std::time::SystemTimeError),
}

/// More structured repository errors that callers can match on explicitly.
/// Use `RepoError::General` for ad-hoc messages; add new variants here as needed.
#[derive(Debug)]
pub enum RepoError {
    /// Generic repository error with a descriptive message
    General(String),
    /// No commits exist in the repository yet (clear and common case)
    NoCommits,
    /// Given hash is not a commit when one was expected
    NotACommit(String),
    /// A file was found outside the repository (strip_prefix failed)
    FileOutsideRepo(std::path::PathBuf),
    /// Problems with index format (signature, padding, utf-8, etc.)
    InvalidIndex(String),
    /// Invalid reference name (contains .., control chars, etc.)
    InvalidRefName(String),
}

impl GritError {
    /// Create a general repository error with a message
    pub fn repo<S: Into<String>>(msg: S) -> Self {
        GritError::RepositoryError(RepoError::General(msg.into()))
    }

    /// No commits error convenience constructor
    pub fn no_commits() -> Self {
        GritError::RepositoryError(RepoError::NoCommits)
    }

    /// Not-a-commit convenience constructor
    pub fn not_a_commit<S: Into<String>>(hash: S) -> Self {
        GritError::RepositoryError(RepoError::NotACommit(hash.into()))
    }

    /// File outside repository convenience constructor
    pub fn file_outside_repo<P: Into<std::path::PathBuf>>(path: P) -> Self {
        GritError::RepositoryError(RepoError::FileOutsideRepo(path.into()))
    }

    /// Invalid index convenience constructor
    pub fn invalid_index<S: Into<String>>(msg: S) -> Self {
        GritError::RepositoryError(RepoError::InvalidIndex(msg.into()))
    }

    /// Invalid reference name convenience constructor
    pub fn invalid_ref_name<S: Into<String>>(msg: S) -> Self {
        GritError::RepositoryError(RepoError::InvalidRefName(msg.into()))
    }
}

impl fmt::Display for GritError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GritError::Io(err) => write!(f, "IO error: {}", err),
            GritError::ObjectNotFound(hash) => write!(f, "Object not found: {}", hash),
            GritError::CorruptObject(msg) => write!(f, "Corrupt object: {}", msg),
            GritError::RepositoryError(repo_err) => match repo_err {
                RepoError::General(msg) => write!(f, "General repository issue: {}", msg),
                RepoError::NoCommits => write!(f, "No commits exist in the repository yet"),
                RepoError::NotACommit(hash) => write!(f, "Not a commit: {}", hash),
                RepoError::FileOutsideRepo(path) => {
                    write!(f, "File outside repository: {}", path.display())
                }
                RepoError::InvalidIndex(msg) => write!(f, "Invalid index: {}", msg),
                RepoError::InvalidRefName(msg) => write!(f, "Invalid ref name: {}", msg),
            },
            GritError::HexDecode(err) => write!(f, "Hex decode error: {}", err),
            GritError::TimeError(err) => write!(f, "Time error: {}", err),
        }
    }
}

impl std::error::Error for GritError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GritError::Io(err) => Some(err),
            GritError::HexDecode(err) => Some(err),
            GritError::TimeError(err) => Some(err),
            _ => None,
        }
    }
}

/// Automatic conversion from std::io::Error to GritError.
/// This allows using the '?' operator with I/O operations throughout the codebase.
impl From<std::io::Error> for GritError {
    fn from(err: std::io::Error) -> Self {
        GritError::Io(err)
    }
}

/// Automatic conversion from hex::FromHexError to GritError.
/// This allows using the '?' operator with hex decoding operations.
impl From<hex::FromHexError> for GritError {
    fn from(err: hex::FromHexError) -> Self {
        GritError::HexDecode(err)
    }
}

/// Automatic conversion from std::time::SystemTimeError to GritError.
/// This allows using the '?' operator with time operations.
impl From<std::time::SystemTimeError> for GritError {
    fn from(err: std::time::SystemTimeError) -> Self {
        GritError::TimeError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error as IoError;
    use std::path::PathBuf;

    #[test]
    fn test_repo_error_display_and_helpers() {
        assert_eq!(
            GritError::repo("oops").to_string(),
            "General repository issue: oops"
        );
        assert_eq!(
            GritError::no_commits().to_string(),
            "No commits exist in the repository yet"
        );
        assert_eq!(
            GritError::not_a_commit("deadbeef").to_string(),
            "Not a commit: deadbeef"
        );
        assert_eq!(
            GritError::file_outside_repo(PathBuf::from("/tmp/foo")).to_string(),
            "File outside repository: /tmp/foo"
        );
        assert_eq!(
            GritError::invalid_index("bad").to_string(),
            "Invalid index: bad"
        );
        assert_eq!(
            GritError::invalid_ref_name("badref").to_string(),
            "Invalid ref name: badref"
        );
    }

    #[test]
    fn test_sources_for_wrapped_errors() {
        // IO source should be exposed
        let io_err = IoError::other("boom");
        let g = GritError::from(io_err);
        assert!(g.source().is_some());

        // Hex decode source should be exposed via a decode attempt
        let hex_err = hex::decode("zzz").unwrap_err();
        let ghex = GritError::from(hex_err);
        assert!(ghex.source().is_some());
    }
}
