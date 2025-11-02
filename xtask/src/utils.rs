use anyhow::{Context, Result};
use camino::Utf8PathBuf;

// URLs
pub const BPP_DOWNLOAD_URL: &str = "https://archive.org/compress/ByondPreservationProject/formats=ZIP&file=/ByondPreservationProject.zip";

// Directory names
pub const WORKSPACE_DIR: &str = "workspace";
pub const PROJECTS_DIR: &str = "projects";
pub const EXTRACTED_DIR: &str = "extracted";
pub const ZIPS_DIR: &str = "zips";

// File names and extensions
pub const BPP_ARCHIVE_NAME: &str = "ByondPreservationProject.zip";
pub const RSC_EXTENSION: &str = "rsc";
pub const ZIP_EXTENSION: &str = "zip";

// Binary name - matches the package name in Cargo.toml
pub const RSC_EXTRACTOR_BIN: &str = "rsc-extractor";

/// Get the workspace root directory (parent of xtask)
pub fn workspace_root() -> Result<Utf8PathBuf> {
    let manifest_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .context("Failed to get workspace root")
}

/// Get the path to the BPP archive
pub fn bpp_archive_path() -> Result<Utf8PathBuf> {
    let workspace_dir = workspace_root()?.join(WORKSPACE_DIR);
    Ok(workspace_dir.join(BPP_ARCHIVE_NAME))
}

/// Build the rsc-extractor binary using current profile and return the runner
pub fn build_rsc_extractor_current() -> Result<escargot::CargoRun> {
    println!("Building {}…", RSC_EXTRACTOR_BIN);
    escargot::CargoBuild::new()
        .bin(RSC_EXTRACTOR_BIN)
        .current_release()
        .manifest_path(workspace_root()?.join("Cargo.toml").as_std_path())
        .run()
        .context("Failed to build rsc-extractor")
}
