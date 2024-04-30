//! Module to run pytest or tox for each mutant in a temporary directory in parallel.
//!
//! This Rust module provides functionalities to execute test suites against Python code mutants. It facilitates
//! the identification of weaknesses in test suites by running them against code variations (mutants) where
//! specific elements have been programmatically altered. It supports running these tests in isolated environments
//! using temporary directories (preferred) or in-place (not preferred), leveraging parallel processing capabilities to enhance performance.
//!
//! ## Features
//!
//! - **Parallel Execution**: Utilizes `rayon` for concurrent execution of tests across multiple mutants.
//! - **Flexible Test Runners**: Supports different test runners like Pytest and Tox, providing versatility in
//!   how Python tests are executed.
//! - **Isolated Test Environments**: Employs `tempfile` for creating temporary directories, ensuring that
//!   test runs do not interfere with each other and the original codebase remains unaltered.
//! - **Detailed Progress Tracking**: Integrates `indicatif` for real-time progress tracking and logging, enhancing
//!   visibility into the testing process.
//! - **Output Customization**: Offers different levels of output verbosity to tailor the feedback from the test
//!   runs according to user preference.
//!
//! ## Usage
//!
//! To use this module, specify the root directory of the Python project, a list of mutants, the desired test
//! runner, and the path to the tests. You can choose the level of output detail and whether to run tests in a
//! temporary directory or in-place.
//!
//! ```no_run
//! use pymute::runner::{Runner, OutputLevel, run_mutants};
//! use pymute::mutants::{find_mutants, MutationType};
//! use std::path::PathBuf;
//!
//! let root = PathBuf::from("path/to/python/project");
//! let mutation_types = &[MutationType::MathOps, MutationType::Booleans];
//! let mutants = find_mutants(glob_pattern, mutation_types).expect("Error finding mutants");
//! let tests = "tests/".to_string();
//! let runner = Runner::Pytest;
//! let output_level = OutputLevel::Process;
//!
//! run_mutants(&root, &mutants, &runner, &tests, &None, &output_level);
//! ```
//!
//! ## Dependencies
//!
//! This module depends on external crates such as `rayon` for parallelism, `tempfile` for managing temporary
//! directories, `indicatif` for progress reporting, and `cp_r` for directory copying.
//!

use crate::mutants::Mutant;
use cp_r::CopyOptions;
use indicatif::{
    self, style::ProgressStyle, ParallelProgressIterator, ProgressBar, ProgressIterator,
};

use clap::ValueEnum;
use rayon::prelude::*;

use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use tempfile::tempdir;

use colored::Colorize;

/// Define the runner to use to run the test suite.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Runner {
    /// Run with Pytest.
    Pytest,
    /// Run with Tox.
    Tox,
}

/// Define the output level when running the tests for mutants.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputLevel {
    /// missed: print out only mutants that were missed by the tests.
    Missed,
    /// caught: print out also mutants that were caught by the tests.
    Caught,
    /// process: print out also output from the individual processes.
    Process,
}

/// Run tests for all mutants each in their own temporary directory.
///
/// Run in parallel using rayon.
///
/// Parameters
/// ----------
/// root: PathBuf to the root of the original python project.
/// mutants: Vec of Mutants for which to run tests in individual sub-processes.
/// runner: Which runner to use to run the test suite.
/// tests: Path to the tests to run via tests as string. Only relevant if the runner
/// is runner::Runner::Pytest.
/// environment: If running via Tox, this environment is passed over to the `-e` option.
/// output_level: How much to print while running the mutant.
pub fn run_mutants(
    root: &PathBuf,
    mutants: &Vec<Mutant>,
    runner: &Runner,
    tests: &String,
    environment: &Option<String>,
    output_level: &OutputLevel,
) {
    let bar = ProgressBar::new(mutants.len().try_into().unwrap());
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );

    mutants
        .par_iter()
        .progress_with(bar.clone())
        .for_each(|mutant| {
            bar.set_message(format!("[{}]: {mutant}\r", "RUNNING".yellow()));
            let result = run_mutant(mutant, root, tests, output_level, runner, environment)
                .unwrap_or_else(|_| panic!("Mutant run failed for {mutant}"));

            match result {
                MutantResult::Missed => {
                    bar.println(format!("[{}] Mutant Survived: {}", "MISSED".red(), mutant));
                }
                _ => {
                    if let OutputLevel::Missed = output_level {
                    } else {
                        bar.println(format!("[{}] Mutant Killed: {}", "CAUGHT".green(), mutant));
                    };
                }
            }
        });
}

/// Run tests for all mutants each in place.
///
/// This is not that well tested yet, and the preferred option is to use `run_mutants`
/// to test each mutant in a temporary directory so that mutants don't affect each
/// other.
///
/// Parameters
/// ----------
/// root: PathBuf to the root of the original python project.
/// mutants: Vec of Mutants for which to run tests in individual sub-processes.
/// runner: Which runner to use to run the test suite.
/// tests: Path to the tests to run via tests as string. Only relevant if the runner
/// is runner::Runner::Pytest.
/// environment: If running via Tox, this environment is passed over to the `-e` option.
/// output_level: How much to print while running the mutant.
pub fn run_mutants_inplace(
    root: &PathBuf,
    mutants: &[Mutant],
    runner: &Runner,
    tests: &String,
    environment: &Option<String>,
    output_level: &OutputLevel,
    num_threads: &Option<usize>,
) {
    let bar = ProgressBar::new(mutants.len().try_into().unwrap());
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );
    mutants
        .iter()
        .progress_with(bar.clone())
        .for_each(|mutant| {
            bar.set_message(format!("[{}]: {mutant}\r", "RUNNING".yellow()));
            let result = run_mutant_inplace(
                mutant,
                root,
                tests,
                output_level,
                runner,
                environment,
                num_threads,
            )
            .unwrap_or_else(|_| panic!("Mutant run failed for {}", mutant));

            match result {
                MutantResult::Missed => {
                    bar.println(format!("[{}] Mutant Survived: {}", "MISSED".red(), mutant));
                }
                _ => {
                    if let OutputLevel::Missed = output_level {
                    } else {
                        bar.println(format!("[{}] Mutant Killed: {}", "CAUGHT".green(), mutant));
                    };
                }
            }
        })
}

/// Run test for one mutant in place.
fn run_mutant_inplace(
    mutant: &Mutant,
    root: &PathBuf,
    tests_glob: &String,
    output_level: &OutputLevel,
    runner: &Runner,
    environment: &Option<String>,
    num_threads: &Option<usize>,
) -> Result<MutantResult, Box<dyn Error>> {
    mutant.insert().expect("Failed to insert the mutant!");

    // build the correct command depending on arguments
    let program = match runner {
        Runner::Pytest => "python",
        Runner::Tox => "tox",
    };
    let mut command = Command::new(program);

    match runner {
        Runner::Pytest => {
            command
                .arg("-B")
                .arg("-m")
                .arg("pytest")
                .arg(tests_glob)
                .arg("-x");
            if let Some(n) = num_threads {
                command.arg(format!("-n {n}"));
            };
        }
        Runner::Tox => {
            if let Some(env) = environment {
                command.arg(format!("-e {env}"));
            };
        }
    };

    match output_level {
        OutputLevel::Process => (),
        _ => {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
    };

    let status = command.current_dir(root).status()?;

    mutant.remove().expect("Failed to remove the mutant!");

    if status.success() {
        Ok(MutantResult::Missed)
    } else {
        Ok(MutantResult::Caught)
    }
}

/// Run tests for one mutant in a temporary directory
fn run_mutant(
    mutant: &Mutant,
    root: &PathBuf,
    tests_glob: &String,
    output_level: &OutputLevel,
    runner: &Runner,
    environment: &Option<String>,
) -> Result<MutantResult, Box<dyn Error>> {
    let dir = tempdir().expect("Failed to create temporary directory!");

    let root_path = root;
    let _stats = CopyOptions::new()
        .copy_tree(root_path, dir.path())
        .expect("Failed to copy the Python project root!");

    mutant
        .insert_in_new_root(root_path, dir.path())
        .expect("Failed to insert mutant");

    // build the correct command depending on arguments
    let program = match runner {
        Runner::Pytest => "python",
        Runner::Tox => "tox",
    };
    let mut command = Command::new(program);

    match runner {
        Runner::Pytest => {
            command
                .arg("-B")
                .arg("-m")
                .arg("pytest")
                .arg(tests_glob)
                .arg("-x");
        }
        Runner::Tox => {
            if let Some(env) = environment {
                command.arg(format!("-e {env}"));
            };
        }
    };

    match output_level {
        OutputLevel::Process => (),
        _ => {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
    };

    let status = command.current_dir(&dir).status()?;

    dir.close().unwrap();

    if status.success() {
        Ok(MutantResult::Missed)
    } else {
        Ok(MutantResult::Caught)
    }
}

enum MutantResult {
    Caught,
    Missed,
}

#[cfg(test)]
mod tests {
    use crate::mutants::{self, MutationType};
    use crate::runner;
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };
    use tempfile::tempdir;

    #[test]
    fn test_pytest_mutants() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();

        let multiline_string_script_1 = "def add(a, b):
    return a + b

# this is a + comment
def sub(a, b):
    return a - b

res = sub(5, 6) * add(7, 8)
print(res) # print the result *
";

        let multiline_string_script_2 = "def div(a, b):
    return a / b

# this is a + comment
def mul(a, b):
    return a * b

res = div(5, 6) - mul(7, 8)
print(res) # print the result +
";
        let multiline_string_script_3 = "def print_number(a, b):
    res = a + b
    print(\"a + b = {res}\")

# this is a + comment

";

        let multiline_string_script_test_1 = "def print_number(a, b):
    res = a + b
    print(\"a + b = {res}\")

# this is a + comment

";
        let multiline_string_script_test_2 = "def print_number(a, b):
    res = a + b
    print(\"a + b = {res}\")

# this is a + comment

";

        // creating a nested directory structure
        let sub_dir1 = base_path.join("dir1");
        let sub_dir1_1 = sub_dir1.join("dir1_1");
        let sub_dir1_1_1 = sub_dir1_1.join("dir1_1_1");

        // ensure all directories are created
        fs::create_dir_all(&sub_dir1_1_1).unwrap();

        let script1 = sub_dir1.join("script1.py");
        let mut script1 = File::create(script1).unwrap();

        write!(script1, "{}", multiline_string_script_1)
            .expect("Failed to write to temporary file");

        // create a decoy txt file that should not be matched
        let decoy = base_path.join("script1.txt");
        let mut decoy = File::create(decoy).unwrap();

        write!(decoy, "{}", multiline_string_script_1).expect("Failed to write txt file.");

        let script2 = sub_dir1_1.join("script2.py");
        let mut script2 = File::create(script2).unwrap();

        write!(script2, "{}", multiline_string_script_2)
            .expect("Failed to write to temporary file");

        let script3 = sub_dir1_1_1.join("script3.py");
        let mut script3 = File::create(script3).unwrap();

        write!(script3, "{}", multiline_string_script_3)
            .expect("Failed to write to temporary file");

        let test_script = sub_dir1_1_1.join("test_script.py");
        let mut test_script = File::create(test_script).unwrap();

        write!(test_script, "{}", multiline_string_script_test_1)
            .expect("Failed to write to temporary file");

        let script_test = sub_dir1_1_1.join("script_test.py");
        let mut script_test = File::create(script_test).unwrap();

        write!(script_test, "{}", multiline_string_script_test_2)
            .expect("Failed to write to temporary file");

        let glob_expr = base_path.to_str().unwrap();
        let glob_expr = format!("{glob_expr}/**/*.py");

        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];
        let mutants_vec = mutants::find_mutants(&glob_expr, &mutation_types).unwrap();

        assert_eq!(mutants_vec.len(), 7);

        runner::run_mutants(
            &PathBuf::from(base_path),
            &mutants_vec,
            &runner::Runner::Pytest,
            &".".into(),
            &None,
            &runner::OutputLevel::Missed,
        );

        temp_dir.close().unwrap();
    }
}
