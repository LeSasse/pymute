use pymute::mutants::find_mutants;
use pymute::runner;
use rand::{seq::IteratorRandom, thread_rng};

use clap::{Parser, ValueEnum};
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
    num_threads: Option<usize>,

    /// Output level of the program
    #[arg(short, long)]
    #[arg(value_enum)]
    #[arg(default_value_t = OutputLevelCli::Missed)]
    output_level: OutputLevelCli,

    /// Test runner to use.
    #[arg(short, long)]
    #[arg(value_enum)]
    #[arg(default_value_t = RunnerCli::Pytest)]
    runner: RunnerCli,

    /// Tox environment to use. Ignored if pytest runner is used.
    #[arg(short, long)]
    #[arg(value_enum)]
    environment: Option<String>,

    /// Maximum number of mutants to be run. If set, will choose a random subset
    /// of n mutants.
    #[arg(long)]
    max_mutants: Option<usize>,

    /// Whether to run inplace.
    #[arg(short, long)]
    pub inplace: bool,

    /// List mutants and exit.
    #[arg(short, long)]
    pub list: bool,
}

fn main() {
    let args = Cli::parse();
    let modules: PathBuf = [&args.root, &args.modules.into()].iter().collect();

    let output_level = match args.output_level {
        OutputLevelCli::Missed => runner::OutputLevel::Missed,
        OutputLevelCli::Caught => runner::OutputLevel::Caught,
        OutputLevelCli::Process => runner::OutputLevel::Process,
    };

    let runner = match args.runner {
        RunnerCli::Pytest => runner::Runner::Pytest,
        RunnerCli::Tox => runner::Runner::Tox,
    };

    let mutants = match args.max_mutants {
        Some(max) => {
            let mut rng = thread_rng();
            find_mutants(
                modules
                    .into_os_string()
                    .to_str()
                    .expect("Invalid Glob Expression?"),
            )
            .expect("Failed to find mutants!")
            .into_iter()
            .choose_multiple(&mut rng, max)
            .into_iter()
            .collect()
        }
        None => find_mutants(
            modules
                .into_os_string()
                .to_str()
                .expect("Invalid Glob Expression?"),
        )
        .expect("Failed to find mutants!"),
    };

    if args.list {
        for mutant in &mutants {
            println!("{mutant}");
        }
        return;
    }

    let _n_mutants = mutants.len();

    if args.inplace {
        runner::run_mutants_inplace(
            &mutants,
            &args.root,
            &args.tests,
            &output_level,
            &runner,
            &args.environment,
            &args.num_threads,
        )
    } else {
        if let Some(n) = args.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .expect("Failed to set the number of threads using rayon.");
        }
        runner::run_mutants(
            &mutants,
            &args.root,
            &args.tests,
            &output_level,
            &runner,
            &args.environment,
        );
    }
}

// Define outout level enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputLevelCli {
    /// missed: print out only mutants that were missed by the tests.
    Missed,
    /// caught: print out also mutants that were caught by the tests.
    Caught,
    /// process: print out also output from the individual processes.
    Process,
}

// Define the test runner.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum RunnerCli {
    /// Pytest
    Pytest,
    /// Tox
    Tox,
}
