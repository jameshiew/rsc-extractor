use anyhow::{Context, Result};
use bstr::ByteSlice as _;
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;

use crate::utils::{
    EXTRACTED_DIR, PROJECTS_DIR, RSC_EXTENSION, WORKSPACE_DIR, build_rsc_extractor_current,
    workspace_root,
};

pub fn analyze_all() -> Result<()> {
    let workspace = workspace_root()?.join(WORKSPACE_DIR);
    let projects_dir = workspace.join(PROJECTS_DIR);

    if !projects_dir.exists() {
        anyhow::bail!(
            "Projects directory not found at {}. Run 'cargo xtask unzip-bpp' first.",
            projects_dir
        );
    }

    println!("Finding all .{} files in {}…", RSC_EXTENSION, projects_dir);

    // Find all .rsc files using walkdir
    let mut rsc_files = Vec::new();
    for entry in WalkDir::new(projects_dir.as_std_path()).follow_links(false) {
        let entry =
            entry.with_context(|| format!("Failed to read directory entry in {}", projects_dir))?;
        if entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case(RSC_EXTENSION))
            .unwrap_or(false)
        {
            // Keep as PathBuf to handle any potential non-UTF-8 paths defensively
            rsc_files.push(entry.path().to_path_buf());
        }
    }

    if rsc_files.is_empty() {
        println!("No .{} files found.", RSC_EXTENSION);
        return Ok(());
    }

    let total_files = rsc_files.len();
    println!("Found {} .{} files to analyze", total_files, RSC_EXTENSION);

    // Build the rsc-extractor binary using escargot
    let cargo_run = build_rsc_extractor_current()?;

    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} .rsc files analyzed",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for rsc_file in rsc_files {
        // Run the built rsc-extractor binary
        let mut command = cargo_run.command();
        let status = command.arg(&rsc_file).status().with_context(|| {
            format!(
                "Failed to run rsc-extractor on {}",
                rsc_file.as_os_str().as_encoded_bytes().to_str_lossy()
            )
        })?;

        if status.success() {
            success_count += 1;
        } else {
            error_count += 1;
        }

        pb.inc(1);
    }

    pb.finish_with_message(format!(
        "✓ Analyzed {} files ({} successful, {} errors)",
        total_files, success_count, error_count
    ));

    Ok(())
}

pub fn extract_all() -> Result<()> {
    let workspace = workspace_root()?.join(WORKSPACE_DIR);
    let projects_dir = workspace.join(PROJECTS_DIR);
    let extracted_dir = workspace.join(EXTRACTED_DIR);

    if !projects_dir.exists() {
        anyhow::bail!(
            "Projects directory not found at {}. Run 'cargo xtask unzip-bpp' first.",
            projects_dir
        );
    }

    println!("Finding all .{} files in {}…", RSC_EXTENSION, projects_dir);

    // Find all .rsc files using walkdir
    let mut rsc_files = Vec::new();
    for entry in WalkDir::new(projects_dir.as_std_path()).follow_links(false) {
        let entry =
            entry.with_context(|| format!("Failed to read directory entry in {}", projects_dir))?;
        if entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case(RSC_EXTENSION))
            .unwrap_or(false)
        {
            // Keep as PathBuf to handle any potential non-UTF-8 paths defensively
            rsc_files.push(entry.path().to_path_buf());
        }
    }

    if rsc_files.is_empty() {
        println!("No .{} files found.", RSC_EXTENSION);
        return Ok(());
    }

    let total_files = rsc_files.len();
    println!("Found {} .{} files to extract", total_files, RSC_EXTENSION);

    // Build the rsc-extractor binary using escargot
    let cargo_run = build_rsc_extractor_current()?;

    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} .rsc files unpacked",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for rsc_file in rsc_files {
        // Calculate the output directory path
        // workspace/projects/a/b/c.rsc -> workspace/extracted/a/b/c/
        let relative_path = rsc_file
            .strip_prefix(projects_dir.as_std_path())
            .with_context(|| {
                format!(
                    "Failed to get relative path for {}",
                    rsc_file.as_os_str().as_encoded_bytes().to_str_lossy()
                )
            })?;

        // Remove the .rsc extension to get the directory name
        let out_subpath = relative_path.with_extension("");
        let out_dir = extracted_dir.as_std_path().join(out_subpath);

        // Create the output directory
        std::fs::create_dir_all(&out_dir).with_context(|| {
            format!(
                "Failed to create output directory {}",
                out_dir.as_os_str().as_encoded_bytes().to_str_lossy()
            )
        })?;

        // Run the built rsc-extractor binary with --silent and --out arguments
        let mut command = cargo_run.command();
        let status = command
            .arg("--silent")
            .arg(&rsc_file)
            .arg("--out")
            .arg(&out_dir)
            .status()
            .with_context(|| {
                format!(
                    "Failed to run rsc-extractor on {}",
                    rsc_file.as_os_str().as_encoded_bytes().to_str_lossy()
                )
            })?;

        if status.success() {
            success_count += 1;
        } else {
            error_count += 1;
        }

        pb.inc(1);
    }

    pb.finish_with_message(format!(
        "✓ Extracted {} files ({} successful, {} errors)",
        total_files, success_count, error_count
    ));

    Ok(())
}
