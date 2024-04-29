//! Module to run pytest for each mutant in a temporary directory in parallel.

use crate::mutants::Mutant;
use cp_r::CopyOptions;
use indicatif::{
    self, style::ProgressStyle, ParallelProgressIterator, ProgressBar, ProgressIterator,
};

use rayon::prelude::*;

use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;

use colored::Colorize;

/// Run pytest for all mutants each in their own temporary directory.
///
/// Run in parallel using rayon.
///
/// Parameters
/// ----------
/// mutants: Vec of Mutants for which to run pytest in individual sub-processes.
/// root: PathBuf to the root of the original python project.
/// tests: Path to the tests to run via pytest as string.
pub fn pytest_mutants(
    mutants: &Vec<Mutant>,
    root: &PathBuf,
    tests: &String,
    output_level: &OutputLevel,
    runner: &Runner,
    environment: &Option<String>,
) {
    //let new_line = "\n";

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
                .expect(format!("Mutant run failed for {mutant}").as_str());

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

pub fn pytest_mutants_inplace(
    mutants: &Vec<Mutant>,
    root: &PathBuf,
    tests: &String,
    output_level: &OutputLevel,
    runner: &Runner,
    environment: &Option<String>,
    num_threads: &usize,
) {
    let bar = ProgressBar::new(mutants.len().try_into().unwrap());
    bar.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}")
            .unwrap(),
    );
    mutants
        .iter()
        .progress_with(bar.clone())
        .for_each(|mutant| {
            let result = run_mutant_inplace(
                mutant,
                root,
                tests,
                output_level,
                runner,
                environment,
                num_threads,
            )
            .expect(format!("Mutant run failed for {}", mutant).as_str());

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

pub enum OutputLevel {
    /// missed: print out only mutants that were missed by the tests.
    Missed,
    /// caught: print out also mutants that were caught by the tests.
    Caught,
    /// process: print out also output from the individual processes.
    Process,
}

pub enum Runner {
    Pytest,
    Tox,
}

fn run_mutant_inplace(
    mutant: &Mutant,
    root: &PathBuf,
    tests_glob: &String,
    output_level: &OutputLevel,
    runner: &Runner,
    environment: &Option<String>,
    num_threads: &usize,
) -> Result<MutantResult, Box<dyn Error>> {
    mutant.insert().expect("Failed to insert the mutant!");
    let output = match (runner, output_level, environment) {
        (Runner::Pytest, OutputLevel::Process, _) => Command::new("python")
            .arg("-B")
            .arg("-m")
            .arg("pytest")
            .arg("-x")
            .arg(format!("-n {}", num_threads))
            .arg("--cache-clear")
            .arg(tests_glob)
            .current_dir(root)
            .status(),
        (Runner::Pytest, _, _) => Command::new("python")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg("-B")
            .arg("-m")
            .arg("pytest")
            .arg("-x")
            .arg(format!("-n {}", num_threads))
            .arg("--cache-clear")
            .arg(tests_glob)
            .current_dir(root)
            .status(),
        (Runner::Tox, OutputLevel::Process, Some(env)) => Command::new("tox")
            .arg(format!("-e {env}"))
            .current_dir(root)
            .status(),
        (Runner::Tox, OutputLevel::Process, None) => Command::new("tox").current_dir(root).status(),
        (Runner::Tox, _, Some(env)) => Command::new("tox")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(format!("-e {env}"))
            .current_dir(root)
            .status(),
        (Runner::Tox, _, None) => Command::new("tox")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(root)
            .status(),
    };

    mutant.remove().expect("Failed to remove the mutant!");
    let status = output?;

    if status.success() {
        Ok(MutantResult::Missed)
    } else {
        Ok(MutantResult::Caught)
    }
}

/// Run pytest for one mutant in a temporary directory
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

    let _ = mutant.insert_in_new_root(root_path, dir.path());

    let output = match (runner, output_level, environment) {
        (Runner::Pytest, OutputLevel::Process, _) => Command::new("pytest")
            .arg("-x")
            .arg(tests_glob)
            .current_dir(&dir)
            .status(),
        (Runner::Pytest, _, _) => Command::new("pytest")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg("-x")
            .arg(tests_glob)
            .current_dir(&dir)
            .status(),
        (Runner::Tox, OutputLevel::Process, Some(env)) => Command::new("tox")
            .arg(format!("-e {env}"))
            .current_dir(&dir)
            .status(),
        (Runner::Tox, OutputLevel::Process, None) => Command::new("tox").current_dir(&dir).status(),
        (Runner::Tox, _, Some(env)) => Command::new("tox")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(format!("-e {env}"))
            .current_dir(&dir)
            .status(),
        (Runner::Tox, _, None) => Command::new("tox")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(&dir)
            .status(),
    };
    dir.close().unwrap();

    let status = output?;

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
    use crate::mutants;
    use crate::pytest;
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
        let mut script1 = File::create(&script1).unwrap();

        write!(script1, "{}", multiline_string_script_1)
            .expect("Failed to write to temporary file");

        // create a decoy txt file that should not be matched
        let decoy = base_path.join("script1.txt");
        let mut decoy = File::create(&decoy).unwrap();

        write!(decoy, "{}", multiline_string_script_1).expect("Failed to write txt file.");

        let script2 = sub_dir1_1.join("script2.py");
        let mut script2 = File::create(&script2).unwrap();

        write!(script2, "{}", multiline_string_script_2)
            .expect("Failed to write to temporary file");

        let script3 = sub_dir1_1_1.join("script3.py");
        let mut script3 = File::create(&script3).unwrap();

        write!(script3, "{}", multiline_string_script_3)
            .expect("Failed to write to temporary file");

        let test_script = sub_dir1_1_1.join("test_script.py");
        let mut test_script = File::create(&test_script).unwrap();

        write!(test_script, "{}", multiline_string_script_test_1)
            .expect("Failed to write to temporary file");

        let script_test = sub_dir1_1_1.join("script_test.py");
        let mut script_test = File::create(&script_test).unwrap();

        write!(script_test, "{}", multiline_string_script_test_2)
            .expect("Failed to write to temporary file");

        let glob_expr = base_path.to_str().unwrap();
        let glob_expr = format!("{glob_expr}/**/*.py");
        let mutants_vec = mutants::find_mutants(&glob_expr).unwrap();

        assert_eq!(mutants_vec.len(), 7);
        pytest::pytest_mutants(&mutants_vec, &PathBuf::from(base_path), &".".into());

        temp_dir.close().unwrap();
    }
}
