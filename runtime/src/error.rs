// KLIK Runtime - Error handling

use std::fmt;

/// KLIK runtime error type
#[derive(Debug)]
pub enum KlikError {
    /// Division by zero
    DivisionByZero,
    /// Index out of bounds
    IndexOutOfBounds { index: usize, length: usize },
    /// Null pointer dereference
    NullDereference,
    /// Stack overflow
    StackOverflow,
    /// Out of memory
    OutOfMemory,
    /// Assertion failure
    AssertionFailed(String),
    /// IO error
    IoError(std::io::Error),
    /// Custom error
    Custom(String),
}

impl fmt::Display for KlikError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KlikError::DivisionByZero => write!(f, "division by zero"),
            KlikError::IndexOutOfBounds { index, length } => {
                write!(f, "index {} out of bounds for length {}", index, length)
            }
            KlikError::NullDereference => write!(f, "null pointer dereference"),
            KlikError::StackOverflow => write!(f, "stack overflow"),
            KlikError::OutOfMemory => write!(f, "out of memory"),
            KlikError::AssertionFailed(msg) => write!(f, "assertion failed: {}", msg),
            KlikError::IoError(e) => write!(f, "IO error: {}", e),
            KlikError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for KlikError {}

impl From<std::io::Error> for KlikError {
    fn from(e: std::io::Error) -> Self {
        KlikError::IoError(e)
    }
}

/// Result type for KLIK runtime operations
pub type KlikResult<T> = Result<T, KlikError>;

/// Panic handler for KLIK programs
pub fn klik_panic(message: &str) -> ! {
    eprintln!("KLIK PANIC: {}", message);
    std::process::exit(1);
}
