//! Provide mutation testing functions for python codebases.

use crate::cache::{read_csv_cache, write_csv_cache};
use crate::mutants::{find_mutants, MutationType};

use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

use std::{error::Error, fmt, path::PathBuf};

pub mod cache;
pub mod mutants;
pub mod runner;

#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &PathBuf,
    modules: &str,
    tests: &str,
    output_level: &runner::OutputLevel,
    runner: &runner::Runner,
    environment: &Option<String>,
    max_mutants: &Option<usize>,
    mutation_types: &[MutationType],
    list: &bool,
    seed: &u64,
) -> Result<String, Box<dyn Error>> {
    let modules: PathBuf = [root, &PathBuf::from(modules)].iter().collect();

    let cache_path: PathBuf = [root, &PathBuf::from(".pymute_cache.csv")].iter().collect();

    // find mutants from the code base
    let mutants = match max_mutants {
        Some(max) => {
            let mut rng = ChaCha8Rng::seed_from_u64(*seed);

            find_mutants(
                modules
                    .into_os_string()
                    .to_str()
                    .ok_or(InvalidGlobExpression {})?,
                mutation_types,
            )?
            .into_iter()
            .choose_multiple(&mut rng, *max)
            .into_iter()
            .collect()
        }
        None => find_mutants(
            modules
                .into_os_string()
                .to_str()
                .ok_or(InvalidGlobExpression {})?,
            mutation_types,
        )?,
    };

    // read the cache of mutants
    // check if we found mutants that have not been cached yet and add them
    let mutants = if cache_path.is_file() {
        let mut cached = read_csv_cache(&cache_path)?;
        for mutant in mutants.iter() {
            if !cached.contains(mutant) {
                cached.push(mutant.clone())
            }
        }
        cached.sort();
        cached
    } else {
        mutants
    };

    if *list {
        for mutant in &mutants {
            println!("{mutant}");
        }
        return Ok(format!("Found and listed {} mutants", mutants.len()));
    }

    let _n_mutants = mutants.len();

    let cached_result =
        runner::run_mutants(root, &mutants, runner, tests, environment, output_level)?;

    write_csv_cache(&cached_result, &cache_path)
}

#[derive(Debug)]
struct InvalidGlobExpression {}

impl Error for InvalidGlobExpression {}
impl fmt::Display for InvalidGlobExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Program interrupted by user!")
    }
}

#[cfg(test)]
mod tests {
    use crate::mutants::MutationType;
    use crate::run;
    use crate::runner;
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

        run(
            &PathBuf::from(base_path),
            "**/*.py",
            ".",
            &runner::OutputLevel::Missed,
            &runner::Runner::Pytest,
            &None,
            &Some(10),
            &vec![
                MutationType::MathOps,
                MutationType::Conjunctions,
                MutationType::Booleans,
                MutationType::ControlFlow,
                MutationType::CompOps,
                MutationType::Numbers,
            ],
            &false,
            &34,
        )
        .unwrap();

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

        run(
            &PathBuf::from(base_path),
            "**/*.py",
            ".",
            &runner::OutputLevel::Missed,
            &runner::Runner::Pytest,
            &None,
            &None,
            &vec![
                MutationType::MathOps,
                MutationType::Conjunctions,
                MutationType::Booleans,
                MutationType::ControlFlow,
                MutationType::CompOps,
                MutationType::Numbers,
            ],
            &false,
            &34,
        )
        .unwrap();

        // best be safe and close it
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_run_with_cache() {
        let multiline_string_script = "def add(a, b):
    return a + b

# this is a + comment
def sub(a, b):
    return a - b

res = sub(5, 6) * add(7, 8)
print(res) # print the result *
";

        let serialised = r#"file_path,line_number,before,after,status
/projects/project/script.py,2, + , - ,NotRun
/projects/project/script.py,65, - , + ,NotRun
"#;

        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let mut script1 = File::create(base_path.join("script.py")).unwrap();
        write!(script1, "{}", multiline_string_script).expect("Failed to write to temporary file");

        let file_path_cache = base_path.join(".pymute_cache.csv");
        {
            let mut file_cache = File::create(&file_path_cache).unwrap();
            write!(file_cache, "{}", serialised).expect("Failed to write to temporary file");
        }

        run(
            &PathBuf::from(base_path),
            "**/*.py",
            ".",
            &runner::OutputLevel::Missed,
            &runner::Runner::Pytest,
            &None,
            &None,
            &vec![
                MutationType::MathOps,
                MutationType::Conjunctions,
                MutationType::Booleans,
                MutationType::ControlFlow,
                MutationType::CompOps,
                MutationType::Numbers,
            ],
            &true,
            &34,
        )
        .unwrap();

        // best be safe and close it
        temp_dir.close().unwrap();
    }
}
