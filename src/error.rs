use std::fmt;
use hex::FromHexError;

/// Represents all possible errors that can occur in the Crust Git plumbing implementation.
/// This enum centralizes error handling to provide consistent error reporting and
/// easy conversion from standard library errors.
#[derive(Debug)]
pub enum CrustError {
    /// An I/O error occurred during file operations (reading, writing, creating directories).
    /// This wraps std::io::Error for seamless error propagation.
    Io(std::io::Error),

    /// The requested object could not be found in the repository.
    /// Contains the hash of the missing object for debugging.
    ObjectNotFound(String),

    /// An object was found but its content is corrupted or malformed.
    /// Contains a descriptive message about what was wrong with the object.
    CorruptObject(String),

    /// A general repository-related error that doesn't fit other categories.
    /// Used for issues like invalid repository state or configuration problems.
    RepositoryError(String),

    /// Error occurred while decoding hex strings.
    /// This wraps hex::FromHexError for hash decoding operations.
    HexDecode(hex::FromHexError),
}

impl fmt::Display for CrustError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CrustError::Io(err) => write!(f, "IO error: {}", err),
            CrustError::ObjectNotFound(hash) => write!(f, "Object not found: {}", hash),
            CrustError::CorruptObject(msg) => write!(f, "Corrupt object: {}", msg),
            CrustError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            CrustError::HexDecode(err) => write!(f, "Hex decode error: {}", err),
        }
    }
}

impl std::error::Error for CrustError {}

/// Automatic conversion from std::io::Error to CrustError.
/// This allows using the '?' operator with I/O operations throughout the codebase.
impl From<std::io::Error> for CrustError {
    fn from(err: std::io::Error) -> Self {
        CrustError::Io(err)
    }
}

/// Automatic conversion from hex::FromHexError to CrustError.
/// This allows using the '?' operator with hex decoding operations.
impl From<hex::FromHexError> for CrustError {
    fn from(err: hex::FromHexError) -> Self {
        CrustError::HexDecode(err)
    }
}
