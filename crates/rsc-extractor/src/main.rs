use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[macro_use]
mod output;

mod display;
mod error;
mod extractor;
mod types;
mod validation;

use extractor::{ProcessorArgs, process_rsc};
use validation::validate_output_directory;

/// Extract and analyze BYOND .rsc (Resource) files.
///
/// RSC files use the RAD (Random Access Data) format and contain cached resources
/// like images, sounds, and other game assets for BYOND games.
#[derive(Parser, Debug)]
#[command(name = "rsc-extractor")]
#[command(about = "A tool to extract and analyze BYOND .rsc files", long_about = None)]
#[command(version)]
struct Args {
    /// Path to the .rsc file to analyze
    #[arg(value_name = "FILE")]
    rsc_file: PathBuf,

    /// If provided, resources will be extracted to this directory
    #[arg(short, long, value_name = "DIR")]
    out: Option<PathBuf>,

    /// Suppress all output (except logs controlled by RUST_LOG)
    #[arg(short, long)]
    silent: bool,
}

fn main() -> Result<()> {
    // Initialize tracing subscriber with default formatting
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Set silent mode if requested
    output::set_silent(args.silent);

    // Validate output directory if provided
    if let Some(ref out_dir) = args.out {
        validate_output_directory(out_dir)?;
    }

    // Process the RSC file (analyze and optionally extract)
    let processor_args = ProcessorArgs {
        rsc_file: args.rsc_file,
        out_dir: args.out,
    };

    process_rsc(&processor_args)?;
    Ok(())
}
