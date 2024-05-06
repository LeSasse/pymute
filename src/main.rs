use clap::Parser;
use colored::Colorize;
use pymute::mutants::MutationType;
use pymute::{run, runner};
use std::{path::PathBuf, process};

/// Pymute: A Mutation Testing Tool for Python/Pytest written in Rust.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Arguments {
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
    /// (i.e. run all tests found under the root). This option is ignored when
    /// running your tests via tox, because tox will run whatever commands
    /// you specify in your `tox.ini` file. Instead set the `--environment` option
    /// to run specific tox test environments.
    #[arg(short, long)]
    #[arg(default_value = ".")]
    tests: String,

    /// Number of threads to run individual mutants in parallel in different
    /// temporary directories.
    #[arg(short, long)]
    #[arg(default_value = "1")]
    num_threads: usize,

    /// Output level of the program
    #[arg(short, long)]
    #[arg(value_enum)]
    #[arg(default_value_t = runner::OutputLevel::Missed)]
    output_level: runner::OutputLevel,

    /// Test runner to use.
    #[arg(short, long)]
    #[arg(value_enum)]
    #[arg(default_value_t = runner::Runner::Pytest)]
    runner: runner::Runner,

    /// Tox environment to use. Ignored if pytest runner is used.
    #[arg(short, long)]
    #[arg(value_enum)]
    environment: Option<String>,

    /// Maximum number of mutants to be run. If set, will choose a random subset
    /// of n mutants. Consider setting the `--seed` option
    #[arg(long)]
    max_mutants: Option<usize>,

    /// Mutation types.
    #[arg(long)]
    #[arg(value_enum)]
    #[arg(default_values_t = [
	MutationType::MathOps,
	MutationType::Conjunctions,
	MutationType::Booleans,
	MutationType::ControlFlow,
	MutationType::CompOps,
	MutationType::Numbers,
    ], value_delimiter=',')]
    mutation_types: Vec<MutationType>,

    /// List mutants and exit.
    #[arg(short, long)]
    list: bool,

    /// Seed for random number generator if max_mutants is set.
    #[arg(short, long)]
    #[arg(default_value = "42")]
    seed: u64,
}

fn main() {
    let args = Arguments::parse();

    match rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
    {
        Ok(_) => {}
        Err(err) => {
            println!("{}: {}", "Error".red(), err);
            process::exit(1);
        }
    }

    match run(
        &args.root,
        &args.modules,
        &args.tests,
        &args.output_level,
        &args.runner,
        &args.environment,
        &args.max_mutants,
        &args.mutation_types,
        &args.list,
        &args.seed,
    ) {
        Ok(msg) => eprintln!("{}: {msg}!", "Success".green()),
        Err(err) => {
            eprintln!("{}: {}", "Error".red(), err);
            process::exit(1);
        }
    };
}
