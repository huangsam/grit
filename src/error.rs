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

    /// A general repository-related error that doesn't fit other categories.
    /// Used for issues like invalid repository state or configuration problems.
    RepositoryError(String),

    /// Error occurred while decoding hex strings.
    /// This wraps hex::FromHexError for hash decoding operations.
    HexDecode(hex::FromHexError),

    /// Error occurred while working with system time.
    /// This wraps std::time::SystemTimeError for timestamp operations.
    TimeError(std::time::SystemTimeError),
}

impl fmt::Display for GritError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GritError::Io(err) => write!(f, "IO error: {}", err),
            GritError::ObjectNotFound(hash) => write!(f, "Object not found: {}", hash),
            GritError::CorruptObject(msg) => write!(f, "Corrupt object: {}", msg),
            GritError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            GritError::HexDecode(err) => write!(f, "Hex decode error: {}", err),
            GritError::TimeError(err) => write!(f, "Time error: {}", err),
        }
    }
}

impl std::error::Error for GritError {}

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
