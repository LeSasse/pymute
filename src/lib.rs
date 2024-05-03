//! Provide mutation testing functions for python codebases.

use crate::mutants::{find_mutants, MutationType};

use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

use clap::Parser;
use std::{error::Error, path::PathBuf};

pub mod mutants;
pub mod runner;

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
    pub num_threads: usize,

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

pub fn run(args: &Arguments) -> Result<(), Box<dyn Error>> {
    let modules: PathBuf = [&args.root, &PathBuf::from(&args.modules)].iter().collect();

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
        return Ok(());
    }

    let _n_mutants = mutants.len();

    runner::run_mutants(
        &args.root,
        &mutants,
        &args.runner,
        &args.tests,
        &args.environment,
        &args.output_level,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::mutants::MutationType;
    use crate::runner;
    use crate::{run, Arguments};
    use std::{fs::File, io::Write, path::PathBuf};
    use tempfile::tempdir;

    #[test]
    fn test_run() {
        let multiline_string_script = "def add(a, b):
    return a + b

# this is a + comment
def sub(a, b):
    return a - b

res = sub(5, 6) * add(7, 8)
print(res) # print the result *
";

        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let mut script1 = File::create(base_path.join("script.py")).unwrap();
        write!(script1, "{}", multiline_string_script).expect("Failed to write to temporary file");

        let arguments = Arguments {
            root: PathBuf::from(base_path),
            modules: "**/*.py".to_string(),
            tests: ".".to_string(),
            num_threads: 1,
            output_level: runner::OutputLevel::Missed,
            runner: runner::Runner::Pytest,
            environment: None,
            max_mutants: Some(10),
            mutation_types: vec![
                MutationType::MathOps,
                MutationType::Conjunctions,
                MutationType::Booleans,
                MutationType::ControlFlow,
                MutationType::CompOps,
                MutationType::Numbers,
            ],
            list: false,
            seed: 34,
        };

        run(&arguments).unwrap();

        // best be safe and close it
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_run_no_max_mutants() {
        let multiline_string_script = "def add(a, b):
    return a + b

# this is a + comment
def sub(a, b):
    return a - b

res = sub(5, 6) * add(7, 8)
print(res) # print the result *
";

        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let mut script1 = File::create(base_path.join("script.py")).unwrap();
        write!(script1, "{}", multiline_string_script).expect("Failed to write to temporary file");

        let arguments = Arguments {
            root: PathBuf::from(base_path),
            modules: "**/*.py".to_string(),
            tests: ".".to_string(),
            num_threads: 1,
            output_level: runner::OutputLevel::Missed,
            runner: runner::Runner::Pytest,
            environment: None,
            max_mutants: None,
            mutation_types: vec![
                MutationType::MathOps,
                MutationType::Conjunctions,
                MutationType::Booleans,
                MutationType::ControlFlow,
                MutationType::CompOps,
                MutationType::Numbers,
            ],
            list: false,
            seed: 34,
        };

        run(&arguments).unwrap();

        // best be safe and close it
        temp_dir.close().unwrap();
    }
}
