use std::collections::HashSet;
use std::io::{BufReader, Cursor, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use binrw::BinRead;
use bstr::ByteSlice as _;
use bytesize::ByteSize;
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use filetime::{FileTime, set_file_times as set_file_times_impl};

use crate::display::{print_rsc_entry, print_summary};
use crate::error::is_eof_error;
use crate::types::{RadEntry, RscEntry};

/// Set file access and modification times based on RSC entry timestamps.
///
/// Uses original_timestamp for mtime (modification time) if available (non-zero),
/// otherwise falls back to timestamp. Uses timestamp for atime (access time).
///
/// # Arguments
/// * `path` - Path to the file (must be absolute or relative to current dir for std::fs compatibility)
/// * `entry` - The RSC entry containing timestamp information
fn set_file_times(path: &Path, entry: &RscEntry) -> Result<()> {
    // Use original_timestamp for mtime if it's non-zero, otherwise use timestamp
    let mtime_seconds = if entry.original_timestamp != 0 {
        entry.original_timestamp
    } else {
        entry.timestamp
    };

    // Use timestamp for atime
    let atime_seconds = entry.timestamp;

    let atime = FileTime::from_unix_time(atime_seconds as i64, 0);
    let mtime = FileTime::from_unix_time(mtime_seconds as i64, 0);

    set_file_times_impl(path, atime, mtime).with_context(|| {
        format!(
            "Failed to set file times for '{}'",
            path.as_os_str().as_encoded_bytes().to_str_lossy()
        )
    })?;

    Ok(())
}

/// Generate a unique filename by appending (1), (2), etc. to avoid collisions
///
/// Examples:
/// - "file.txt" -> "file (1).txt"
/// - "file (1).txt" -> "file (2).txt"
/// - "filename" (no extension) -> "filename (1)"
fn generate_unique_filename(original: &str, existing: &HashSet<String>) -> String {
    // Split filename into base and extension
    let (base, ext) = if let Some(dot_pos) = original.rfind('.') {
        // Has extension
        let base = &original[..dot_pos];
        let ext = &original[dot_pos..]; // includes the dot
        (base, Some(ext))
    } else {
        // No extension
        (original, None)
    };

    // Try incrementing numbers until we find an unused filename
    let mut counter = 1;
    loop {
        let candidate = if let Some(extension) = ext {
            format!("{} ({}){}", base, counter, extension)
        } else {
            format!("{} ({})", base, counter)
        };

        if !existing.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

/// Arguments for processing an RSC file
pub struct ProcessorArgs {
    pub rsc_file: PathBuf,
    pub out_dir: Option<PathBuf>,
}

/// Main execution function that processes an RSC file.
///
/// This function analyzes the RSC file and prints information about its contents.
/// If an output directory is provided, it also extracts the resources to disk.
pub fn process_rsc(args: &ProcessorArgs) -> Result<()> {
    oprintln!(
        "Processing RSC file: {}",
        args.rsc_file.as_os_str().as_encoded_bytes().to_str_lossy()
    );
    oprintln!();

    // Open the input file using std::fs since we're reading a user-provided path
    let file = std::fs::File::open(&args.rsc_file).with_context(|| {
        format!(
            "Failed to open file '{}'",
            args.rsc_file.as_os_str().as_encoded_bytes().to_str_lossy()
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut entry_num = 0;
    let mut valid_entries = 0;
    let mut invalid_entries = 0;
    let mut encrypted_entries = 0;
    let mut written_files = 0;
    let mut skipped_files = 0;

    // Track filenames to detect collisions
    let mut written_filenames = HashSet::new();

    // Track invalid entry numbers
    let mut invalid_entry_numbers: Vec<usize> = Vec::new();

    // Open the output directory once before the loop if specified
    let output_dir = if let Some(ref out_dir) = args.out_dir {
        Some(
            Dir::open_ambient_dir(out_dir, ambient_authority()).with_context(|| {
                format!(
                    "Failed to open output directory '{}'",
                    out_dir.as_os_str().as_encoded_bytes().to_str_lossy()
                )
            })?,
        )
    } else {
        None
    };

    // Parse entries until we reach EOF
    loop {
        // Try to read the next RAD entry
        let rad_entry = match RadEntry::read(&mut reader) {
            Ok(entry) => entry,
            Err(e) => {
                // Check if we've reached the end of file gracefully
                if is_eof_error(&e) {
                    break;
                }
                return Err(e).with_context(|| {
                    format!("Failed to read RAD entry at position {}", entry_num + 1)
                });
            }
        };

        entry_num += 1;

        // Check if the entry is valid
        if !rad_entry.is_valid() {
            invalid_entries += 1;
            invalid_entry_numbers.push(entry_num);
            skipped_files += 1;
            oprintln!(
                "[INVALID] Entry {} (length: {}, valid: 0x{:02X})",
                entry_num,
                ByteSize::b(rad_entry.entry_length as u64),
                rad_entry.valid
            );
            oprintln!();
            continue;
        }

        valid_entries += 1;

        // Parse the RSC entry from the content
        let mut cursor = Cursor::new(&rad_entry.content);

        let mut rsc_entry = RscEntry::read(&mut cursor).with_context(|| {
            format!(
                "Failed to parse RSC entry {} (RAD length: {})",
                entry_num,
                ByteSize::b(rad_entry.entry_length as u64)
            )
        })?;

        // Infer the MIME type from the data
        rsc_entry.infer_mime_type();

        // Track encrypted entries
        if rsc_entry.is_encrypted() {
            encrypted_entries += 1;
        }

        print_rsc_entry(entry_num, &rsc_entry);

        // Write file if output directory is specified
        if let (Some(dir), Some(base_path)) = (&output_dir, &args.out_dir) {
            match write_entry_to_file(
                dir,
                base_path,
                entry_num,
                &rsc_entry,
                &mut written_filenames,
            ) {
                Ok(true) => written_files += 1,
                Ok(false) => skipped_files += 1,
                Err(e) => {
                    tracing::warn!("Failed to write entry {}: {}", entry_num, e);
                    skipped_files += 1;
                }
            }
        }
    }

    print_summary(
        entry_num,
        valid_entries,
        invalid_entries,
        encrypted_entries,
        &invalid_entry_numbers,
    );

    // Print file writing summary if output directory was specified
    if args.out_dir.is_some() {
        oprintln!();
        oprintln!("Entries extracted: {}", written_files);
        oprintln!("Entries skipped: {}", skipped_files);
    }

    Ok(())
}

/// Write an RSC entry to a file in the output directory
///
/// Returns Ok(true) if the file was written successfully,
/// Ok(false) if the file was skipped (no filename, collision, etc.),
/// or Err if there was an error writing the file.
fn write_entry_to_file(
    out_dir: &Dir,
    base_path: &Path,
    entry_num: usize,
    entry: &RscEntry,
    written_filenames: &mut HashSet<String>,
) -> Result<bool> {
    // Get the filename from the entry
    let filename_str = entry.filename.to_string();

    // Handle entries with no filename
    if filename_str.is_empty() {
        // Create subdirectory for entries with no filename
        let subdir_name = "entries_with_no_filename";
        out_dir
            .create_dir_all(subdir_name)
            .with_context(|| format!("Failed to create directory '{}'", subdir_name))?;

        let no_filename_dir = out_dir
            .open_dir(subdir_name)
            .with_context(|| format!("Failed to open directory '{}'", subdir_name))?;

        // Generate filename with extension based on inferred MIME type
        let filename = if let Some(ext) = entry.extension_from_mime() {
            format!("{}{}", entry_num, ext)
        } else {
            format!("{}", entry_num)
        };

        tracing::warn!(
            "Entry {} (type {}) had no filename, wrote to entries_with_no_filename/{}",
            entry_num,
            entry.get_entry_type(),
            filename,
        );

        // Create and write the file
        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        let mut file = no_filename_dir
            .open_with(&filename, &opts)
            .with_context(|| format!("Failed to create file '{}/{}'", subdir_name, filename))?;

        file.write_all(&entry.data).with_context(|| {
            format!(
                "Failed to write data to file '{}/{}'",
                subdir_name, filename
            )
        })?;

        // Set file timestamps
        let full_path = base_path.join(subdir_name).join(&filename);
        if let Err(e) = set_file_times(&full_path, entry) {
            tracing::warn!(
                "Failed to set file times for {}/{}: {}",
                subdir_name,
                filename,
                e
            );
        }

        written_filenames.insert(filename.clone());
        tracing::debug!("Wrote file: {}/{}", subdir_name, filename);
        return Ok(true);
    }

    // Extract just the filename component (last part after any slashes)
    // This is critical for security - we collapse any path to just the filename
    let filename_component = Path::new(&filename_str)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename_str);

    // Sanitize the filename component
    let sanitized_filename = sanitize_filename::sanitize(filename_component);

    // Check if sanitization resulted in an empty string
    if sanitized_filename.is_empty() {
        tracing::warn!(
            "Filename '{}' resulted in empty string after sanitization, skipping file output",
            filename_str
        );
        return Ok(false);
    }

    // Check for filename collision and generate unique filename if needed
    let final_filename = if written_filenames.contains(&sanitized_filename) {
        let unique_filename = generate_unique_filename(&sanitized_filename, written_filenames);

        if sanitized_filename != filename_component {
            tracing::warn!(
                "Filename collision detected: '{}' (sanitized from '{}'), writing as '{}'",
                sanitized_filename,
                filename_str,
                unique_filename
            );
        } else {
            tracing::warn!(
                "Filename collision detected for '{}', writing as '{}'",
                sanitized_filename,
                unique_filename
            );
        }
        unique_filename
    } else {
        sanitized_filename
    };

    // Warn if writing encrypted data
    if entry.is_encrypted() {
        tracing::warn!(
            "Writing encrypted data to '{}' - file will not be directly usable",
            final_filename
        );
    }

    // Create and write the file using cap-std
    // By using the Dir handle, we can only create files within the output directory
    // Even if an attacker provides ../../etc/passwd, it will be collapsed to just "passwd"
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    let mut file = out_dir
        .open_with(&final_filename, &opts)
        .with_context(|| format!("Failed to create file '{}'", final_filename))?;

    file.write_all(&entry.data)
        .with_context(|| format!("Failed to write data to file '{}'", final_filename))?;

    // Set file timestamps
    let full_path = base_path.join(&final_filename);
    if let Err(e) = set_file_times(&full_path, entry) {
        tracing::warn!("Failed to set file times for {}: {}", final_filename, e);
    }

    // Track the written filename
    written_filenames.insert(final_filename.clone());

    tracing::debug!("Wrote file: {}", final_filename);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_filename_with_extension() {
        let mut existing = HashSet::new();
        existing.insert("file.txt".to_string());

        let result = generate_unique_filename("file.txt", &existing);
        assert_eq!(result, "file (1).txt");

        // Test with multiple collisions
        existing.insert("file (1).txt".to_string());
        let result = generate_unique_filename("file.txt", &existing);
        assert_eq!(result, "file (2).txt");

        existing.insert("file (2).txt".to_string());
        let result = generate_unique_filename("file.txt", &existing);
        assert_eq!(result, "file (3).txt");
    }

    #[test]
    fn test_generate_unique_filename_without_extension() {
        let mut existing = HashSet::new();
        existing.insert("filename".to_string());

        let result = generate_unique_filename("filename", &existing);
        assert_eq!(result, "filename (1)");

        // Test with multiple collisions
        existing.insert("filename (1)".to_string());
        let result = generate_unique_filename("filename", &existing);
        assert_eq!(result, "filename (2)");
    }

    #[test]
    fn test_generate_unique_filename_with_dmi_extension() {
        let mut existing = HashSet::new();
        existing.insert("sprite.dmi".to_string());

        let result = generate_unique_filename("sprite.dmi", &existing);
        assert_eq!(result, "sprite (1).dmi");
    }

    #[test]
    fn test_generate_unique_filename_with_multiple_dots() {
        let mut existing = HashSet::new();
        existing.insert("file.tar.gz".to_string());

        // Should treat .gz as the extension
        let result = generate_unique_filename("file.tar.gz", &existing);
        assert_eq!(result, "file.tar (1).gz");
    }

    #[test]
    fn test_generate_unique_filename_no_collision() {
        let existing = HashSet::new();

        // If there's no collision in the set, it should still generate (1)
        let result = generate_unique_filename("new_file.txt", &existing);
        assert_eq!(result, "new_file (1).txt");
    }
}
