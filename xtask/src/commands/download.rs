use std::io::{Write, copy};
use std::time::Duration;

use anyhow::{Context, Result};
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use cap_tempfile::TempFile;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};

use crate::utils::bpp_archive_path;

/// Helper for writing with progress tracking
struct ProgressWriter<W> {
    inner: W,
    pb: ProgressBar,
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.pb.inc(n as u64);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Create a progress bar for downloads
fn create_download_progress_bar(content_length: Option<u64>) -> ProgressBar {
    match content_length {
        Some(n) => {
            let pb = ProgressBar::new(n);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} \
                     ({bytes_per_sec}) ETA {eta}",
                )
                .unwrap()
                .progress_chars("=> "),
            );
            println!("File size: {}", HumanBytes(n));
            pb
        }
        None => {
            // Unknown size: use an indeterminate spinner
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner} [{elapsed_precise}] Downloading… {bytes} ({bytes_per_sec})",
                )
                .unwrap(),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        }
    }
}

pub fn download_bpp(url: String) -> Result<()> {
    let output_path = bpp_archive_path()?;

    // Check if file already exists
    if output_path.exists() {
        anyhow::bail!(
            "Archive already exists at {}. Delete it first if you want to re-download.",
            output_path
        );
    }

    // Create workspace directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create workspace directory")?;
    }

    println!("Downloading BYOND Preservation Project…");
    println!("URL: {}", url);
    println!("Destination: {}", output_path);

    // Create a client with longer timeout for large files
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3600)) // 1 hour timeout
        .build()
        .context("Failed to build HTTP client")?;

    // Start the request
    let mut response = client.get(url).send().context("Failed to send request")?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed with status: {}", response.status());
    }

    // Open the parent directory for capability-based operations
    let parent_dir = output_path
        .parent()
        .context("Output path has no parent directory")?;
    let file_name = output_path
        .file_name()
        .context("Output path has no file name")?;

    let output_dir = Dir::open_ambient_dir(parent_dir.as_std_path(), ambient_authority())
        .context("Failed to open output directory")?;

    // Create a temporary file in the output directory
    let temp_file = TempFile::new(&output_dir).context("Failed to create temporary file")?;

    // Progress bar setup
    let pb = create_download_progress_bar(response.content_length());

    let mut writer = ProgressWriter {
        inner: temp_file,
        pb: pb.clone(),
    };

    // Stream response -> file with progress
    let bytes_copied = copy(&mut response, &mut writer).context("Failed to write to file")?;

    // Ensure all data is written to disk and replace at the final destination
    let temp_file = writer.inner;
    temp_file
        .replace(file_name)
        .context("Failed to persist temporary file to final destination")?;

    pb.finish_with_message(format!(
        "✓ Downloaded {} to {}",
        HumanBytes(bytes_copied),
        output_path
    ));

    Ok(())
}
