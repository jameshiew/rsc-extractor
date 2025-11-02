use std::path::Path;

use bstr::ByteSlice as _;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use cap_tempfile::TempFile;

use crate::error::ValidationError;

/// Validate that the output directory exists and is writable.
/// Creates the directory if it doesn't exist.
pub fn validate_output_directory(path: &Path) -> Result<(), ValidationError> {
    // Check if the path exists
    if !path.exists() {
        // Try to create the directory using std::fs for initial setup
        // (validation happens before we get a Dir handle)
        std::fs::create_dir_all(path).map_err(|e| ValidationError::NotWritable {
            path: path
                .as_os_str()
                .as_encoded_bytes()
                .to_str_lossy()
                .into_owned(),
            source: e,
        })?;
        tracing::info!(
            "Created output directory: {}",
            path.as_os_str().as_encoded_bytes().to_str_lossy()
        );
    } else {
        // Check if it's a directory
        if !path.is_dir() {
            return Err(ValidationError::NotADir(
                path.as_os_str()
                    .as_encoded_bytes()
                    .to_str_lossy()
                    .into_owned(),
            ));
        }
    }

    // Check if it's writable by opening it with cap-std and creating a temp file
    let dir = Dir::open_ambient_dir(path, ambient_authority()).map_err(|e| {
        ValidationError::NotWritable {
            path: path
                .as_os_str()
                .as_encoded_bytes()
                .to_str_lossy()
                .into_owned(),
            source: e,
        }
    })?;

    // Verify writability by creating a temporary file
    TempFile::new(&dir).map_err(|e| ValidationError::NotWritable {
        path: path
            .as_os_str()
            .as_encoded_bytes()
            .to_str_lossy()
            .into_owned(),
        source: e,
    })?;

    Ok(())
}
