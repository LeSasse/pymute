use pymute::mutants::{find_mutants, MutationType};
use pymute::runner;

use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

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
    num_threads: Option<usize>,

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

    /// Whether to run inplace. For now not the recommended way of running pymute.
    #[arg(short, long)]
    inplace: bool,

    /// List mutants and exit.
    #[arg(short, long)]
    list: bool,

    /// Seed for random number generator if max_mutants is set.
    #[arg(short, long)]
    #[arg(default_value = "42")]
    seed: u64,
}

fn main() {
    let args = Cli::parse();
    let modules: PathBuf = [&args.root, &args.modules.into()].iter().collect();

    let mutants = match args.max_mutants {
        Some(max) => {
            let mut rng = ChaCha8Rng::seed_from_u64(args.seed);

            find_mutants(
                modules
                    .into_os_string()
                    .to_str()
                    .expect("Invalid Glob Expression?"),
                &args.mutation_types,
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
            &args.mutation_types,
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
            &args.root,
            &mutants,
            &args.runner,
            &args.tests,
            &args.environment,
            &args.output_level,
            &args.num_threads,
        )
    } else if let Some(n) = args.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .expect("Failed to set the number of threads using rayon.");
        runner::run_mutants(
            &args.root,
            &mutants,
            &args.runner,
            &args.tests,
            &args.environment,
            &args.output_level,
        );
    } else {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global()
            .expect("Failed to set the number of threads using rayon.");
        runner::run_mutants(
            &args.root,
            &mutants,
            &args.runner,
            &args.tests,
            &args.environment,
            &args.output_level,
        );
    }
}
