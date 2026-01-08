//! Error types for pdf_rs

use thiserror::Error;

/// Result type alias for pdf_rs operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for PDF operations
#[derive(Error, Debug)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error
    #[error("Parse error: {message} at position {position}")]
    Parse { message: String, position: usize },

    /// Invalid PDF structure
    #[error("Invalid PDF structure: {0}")]
    InvalidStructure(String),

    /// Missing required object
    #[error("Missing required object: {0}")]
    MissingObject(String),

    /// Invalid object type
    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Font error
    #[error("Font error: {0}")]
    Font(String),

    /// Image error
    #[error("Image error: {0}")]
    Image(String),
}
