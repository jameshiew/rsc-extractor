use anyhow::{Context, Result};
use bstr::ByteSlice as _;
use camino::{Utf8Path, Utf8PathBuf};
use indicatif::{ProgressBar, ProgressStyle};
use zip::ZipArchive;

use crate::utils::{
    PROJECTS_DIR, WORKSPACE_DIR, ZIP_EXTENSION, ZIPS_DIR, bpp_archive_path, workspace_root,
};

/// Extract all ZIP files found in a directory to the projects directory
fn extract_nested_zips(source_dir: &Utf8Path, projects_dir: &Utf8Path) -> Result<()> {
    // Find all .zip files recursively
    let mut zip_files = Vec::new();
    for entry in walkdir::WalkDir::new(source_dir.as_std_path()) {
        let entry =
            entry.with_context(|| format!("Failed to read directory entry in {}", source_dir))?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some(ZIP_EXTENSION) {
            // Workspace paths should always be UTF-8 - fail if not
            let path = Utf8PathBuf::try_from(entry.path().to_path_buf()).with_context(|| {
                format!(
                    "Path contains invalid UTF-8: {}",
                    entry.path().as_os_str().as_encoded_bytes().to_str_lossy()
                )
            })?;
            zip_files.push(path);
        }
    }

    if zip_files.is_empty() {
        println!("No ZIP files found to extract.");
        return Ok(());
    }

    let total_zips = zip_files.len();
    println!("Found {} ZIP files to extract", total_zips);

    let pb = ProgressBar::new(total_zips as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} projects extracted",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    for zip_path in zip_files {
        // Create a subdirectory in projects based on the zip file name (without extension)
        let project_name = match zip_path.file_stem() {
            Some(name) => name,
            None => {
                eprintln!("⚠ Warning: Failed to get file stem for {}", zip_path);
                pb.inc(1);
                continue;
            }
        };
        let project_dir = projects_dir.join(project_name);

        // Skip if already extracted
        if project_dir.exists() {
            pb.inc(1);
            continue;
        }

        // Extract the ZIP file
        let file = match std::fs::File::open(&zip_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("⚠ Warning: Failed to open {}: {}", zip_path, e);
                pb.inc(1);
                continue;
            }
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("⚠ Warning: Failed to read ZIP archive {}: {}", zip_path, e);
                pb.inc(1);
                continue;
            }
        };

        if let Err(e) = archive.extract(&project_dir) {
            eprintln!(
                "⚠ Warning: Failed to extract {} to {}: {}",
                zip_path, project_dir, e
            );
            pb.inc(1);
            continue;
        }

        pb.inc(1);
    }

    pb.finish_with_message(format!(
        "✓ Extracted {} projects to {}",
        total_zips, projects_dir
    ));

    Ok(())
}

pub fn unzip_bpp() -> Result<()> {
    let archive_path = bpp_archive_path()?;
    let workspace = workspace_root()?.join(WORKSPACE_DIR);
    let extract_dir = workspace.join(ZIPS_DIR);
    let projects_dir = workspace.join(PROJECTS_DIR);

    if !archive_path.exists() {
        anyhow::bail!(
            "Archive not found at {}. Run 'cargo xtask download-bpp' first.",
            archive_path
        );
    }

    // Create extraction directories
    std::fs::create_dir_all(&extract_dir).context("Failed to create extraction directory")?;
    std::fs::create_dir_all(&projects_dir).context("Failed to create projects directory")?;

    println!("Extracting BYOND Preservation Project…");
    println!("Source: {}", archive_path);
    println!("Destination: {}", extract_dir);

    let file = std::fs::File::open(&archive_path).context("Failed to open archive")?;
    let mut archive = ZipArchive::new(file).context("Failed to read ZIP archive")?;

    let total_files = archive.len();
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files unpacked from archive",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    println!("Total files in archive: {}", total_files);

    // Use the built-in extract method
    archive
        .extract(&extract_dir)
        .context("Failed to extract archive")?;
    pb.finish_with_message(format!(
        "✓ Extracted {} files to {}",
        total_files, extract_dir
    ));

    // Second step: Extract nested ZIP files to projects directory
    println!("\nExtracting nested ZIP files to projects…");
    extract_nested_zips(&extract_dir, &projects_dir)?;

    Ok(())
}
