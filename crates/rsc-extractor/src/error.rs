use std::io::ErrorKind;

use thiserror::Error;

/// Domain-specific validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Output path is not a directory: {0}")]
    NotADir(String),

    #[error("Output directory is not writable: {path} ({source})")]
    NotWritable {
        path: String,
        source: std::io::Error,
    },
}

/// Check if a binrw error represents an EOF condition
pub fn is_eof_error(error: &binrw::Error) -> bool {
    match error {
        binrw::Error::Io(io_err) => io_err.kind() == ErrorKind::UnexpectedEof,
        _ => {
            // Also check the string representation for compatibility
            let error_str = error.to_string();
            error_str.contains("UnexpectedEof") || error_str.contains("failed to fill whole buffer")
        }
    }
}
