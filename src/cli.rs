use clap::Parser;
use std::path::PathBuf;

/// Define command-line arguments.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    root: PathBuf,
}
