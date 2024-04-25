use pymute::mutants::find_mutants;
use pymute::pytest;

use clap::Parser;
use std::path::PathBuf;

/// Pymute: A Mutation Testing Tool for Python/Pytest written in Rust.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Define the path to the root of the python project.
    root: PathBuf,

    /// Glob expression to modules for which
    /// mutants should be created. This should be
    /// relative from the root of the python project.
    /// By default, it will take all modules under the root.
    /// Pymute also filters out files that start with
    /// "test_" and end with "_test.py" to avoid scanning
    /// tests for mutants.
    #[arg(short, long)]
    #[arg(default_value = "**/*.py")]
    modules: String,

    /// Path for tests that should be run. This should be
    /// relative from the root of the python project.
    /// By default, it will simply use "."
    /// (i.e. run all tests found under the root).
    #[arg(short, long)]
    #[arg(default_value = ".")]
    tests: String,

    #[arg(short, long)]
    #[arg(default_value = "1")]
    num_threads: usize,
}

fn main() {
    let args = Cli::parse();
    let modules: PathBuf = [&args.root, &args.modules.into()].iter().collect();
    let mutants = find_mutants(
        modules
            .into_os_string()
            .to_str()
            .expect("Invalid Glob Expression?"),
    )
    .expect("Failed to find mutants!");

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
        .expect("Failed to set the number of threads using rayon.");
    pytest::pytest_mutants(&mutants, &args.root, &args.tests);
}
