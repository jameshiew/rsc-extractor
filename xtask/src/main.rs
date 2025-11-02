use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;
mod utils;

use commands::{analyze, download, extract};

use crate::utils::BPP_DOWNLOAD_URL;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for rsc-extractor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Download the BYOND Preservation Project archive
    DownloadBpp,
    /// Extract the BYOND Preservation Project archive
    UnzipBpp,
    /// Analyze all .rsc files in workspace/projects
    AnalyzeAll,
    /// Extract all .rsc files in workspace/projects to workspace/extracted
    ExtractAll,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::DownloadBpp => download::download_bpp(BPP_DOWNLOAD_URL.to_owned())?,
        Commands::UnzipBpp => extract::unzip_bpp()?,
        Commands::AnalyzeAll => analyze::analyze_all()?,
        Commands::ExtractAll => analyze::extract_all()?,
    }

    Ok(())
}
