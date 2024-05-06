//! Provide mutation testing functions for python codebases.

use crate::mutants::{find_mutants, Mutant, MutationType};

use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

use std::{error::Error, fmt, fs::File, io, path::PathBuf};

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

    let file = File::create(cache_path)?;

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

    if *list {
        for mutant in &mutants {
            println!("{mutant}");
        }
        return Ok(format!("Found and listed {} mutants", mutants.len()));
    }

    let _n_mutants = mutants.len();

    let cached_result =
        runner::run_mutants(root, &mutants, runner, tests, environment, output_level)?;

    let mut wtr = csv::Writer::from_writer(file);

    for (i, (status, mutant)) in cached_result.into_iter().enumerate() {
        let mut mutant_result = mutant.clone();
        mutant_result.status = Box::new(status.clone());
        wtr.serialize(mutant_result)?;
    }
    Ok("Results written to cache".to_string())
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
}
