//! # Mutant Generation Module
//!
//! This module provides functionality to identify and mutate specific parts of Python codebases
//! based on predefined mutation criteria. It supports various mutation types including arithmetic
//! operations, boolean expressions, control flow alterations, and more. The primary purpose of this
//! module is to assist in mutation testing by generating potential code mutants, helping developers
//! and testers to identify how well a test suite can detect injected faults.
//!
//! ## Features
//!
//! - **Mutation Identification**: Scans Python files to identify possible points for mutation
//!   based on the specified mutation types.
//! - **Mutation Application**: Capable of applying mutations directly to the code, thereby
//!   generating different mutant variants which can be used for testing the effectiveness of
//!   test suites.
//! - **Support for Multiple Mutation Types**: Handles a variety of mutation types including,
//!   but not limited to, mathematical operations, boolean logic mutations, and control flow changes.
//!
//! ## Usage
//!
//! The main entry points of this module are:
//! - `find_mutants(glob_expression, mutation_types)`: Scans files matching the glob pattern and identifies
//!   potential mutants based on the provided mutation types.
//! - `Mutant::insert()`, `Mutant::insert_in_new_root()`, and `Mutant::remove()`: Methods to apply or remove
//!   mutations on the code files.
//!
//! Ensure that the `glob` crate is correctly configured and that the path specifications align with the
//! target filesystem structure.
//!
//! ## Example
//!
//! To use this module to find and apply mutations in a temporary directory (preferred way):
//!
//! ```
//! use pymute::mutants::{MutationType, find_mutants};
//! use cp_r::CopyOptions;
//! use std::path::PathBuf;
//! use tempfile::tempdir;
//!
//! let project_root = PathBuf::from(".");
//! let glob_pattern = "my_module/**/*.py";
//! let mutation_types = &[MutationType::MathOps, MutationType::Booleans];
//! let mutants = find_mutants(glob_pattern, mutation_types).expect("Error finding mutants");
//!
//! for mutant in mutants {
//!     let dir = tempdir().expect("Failed to create temporary directory!");
//!     mutant.insert_in_new_root(&project_root, dir.path()).expect("Error inserting mutant");
//!     mutant.remove().expect("Error removing mutant");
//!     dir.close().unwrap();
//! }
//! ```
//!
//! To use this module to find and apply mutations in place (removal is not well-tested and reliable as of yet):
//!
//! ```
//! use pymute::mutants::{find_mutants, MutationType};
//!
//! let glob_pattern = "my_module/**/*.py";
//! let mutation_types = &[MutationType::MathOps, MutationType::Booleans];
//! let mutants = find_mutants(glob_pattern, mutation_types).expect("Error finding mutants");
//!
//! for mutant in mutants {
//!     mutant.insert().expect("Error inserting mutant");
//!     mutant.remove().expect("Error removing mutant")
//! }
//! ```
//!
//! ## Dependencies
//!
//! This module depends on external crates such as `glob` for file pattern matching, `regex` for text
//! manipulation, and `colored` for enhancing output readability by coloring text.
//!

use clap::ValueEnum;
use colored::Colorize;
use glob::glob;
use regex::Regex;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// A semantic grouping of different types of possible mutations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum MutationType {
    /// Mutate mathematical operators (e.g. "*,+,-,/")
    MathOps,
    /// Mutate conjunctions in boolean expressions (e.g. "and/or").
    Conjunctions,
    /// Mutate booleans (e.g. "True/False").
    Booleans,
    /// Mutate control flow statements (e.g. if statements).
    ControlFlow,
    /// Mutate comparison operators (e.g. "<,>,==,!=").
    CompOps,
    /// Mutate numbers (e.g. off-by-one errors)
    Numbers,
}

/// Find potential python mutants from files that match the glob expression.
///
/// It will ignore any files that start with test_* and that end with *_test.py
/// to avoid mutating pytest tests.
///
/// Parameters
/// ----------
/// glob_expression: &str compatible with the `glob` crate.
/// mutation_types: Collection of MutationType. Each of the mutation types specified
/// here will be used.
pub fn find_mutants(
    glob_expression: &str,
    mutation_types: &[MutationType],
) -> Result<Vec<Mutant>, Box<dyn Error>> {
    let mut possible_mutants = Vec::<Mutant>::new();

    let replacements = build_replacements(mutation_types);

    for entry in glob(glob_expression).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let file_name = match path.file_name() {
                    Some(f) => f,
                    None => continue,
                };
                let file_name = match file_name.to_str() {
                    Some(f) => f,
                    None => continue,
                };
                if file_name.starts_with("test_") {
                    continue;
                }
                if file_name.ends_with("_test.py") {
                    continue;
                }
                let _ = add_mutants_from_file(&mut possible_mutants, &path, &replacements);
            }
            Err(_e) => {}
        }
    }

    Ok(possible_mutants)
}

/// Define parameters of a potential mutant for a python program.
#[derive(Debug)]
pub struct Mutant {
    /// Path to python file that can be mutated.
    pub file_path: PathBuf,
    /// Line number on which to insert the mutant.
    pub line_number: usize,
    /// The original string.
    pub before: String,
    /// The replacement string.
    pub after: String,
    /// The line before inserting the mutant.
    old_line: String,
}

impl Mutant {
    /// Actually insert the mutant into a file.
    ///
    /// This will take the mutant and insert it in a copy of the python project.
    ///
    /// Parameters
    /// ----------
    /// root: This is the path to the root of the original directory. The root
    /// path will be stripped from the mutants file path.
    /// new_root: This is the path to the root of the copied python project.
    /// The mutant file path will be joined into this one after stripping the original
    /// root. The mutant is then inserted into the copied version of the file
    /// where the potential mutant was found (i.e. it will be inserted into
    /// new_root / mutant_file_path_stripped_of_root)
    pub fn insert_in_new_root(&self, root: &Path, new_root: &Path) -> Result<(), Box<dyn Error>> {
        let abs_path_file = self
            .file_path
            .canonicalize()
            .expect("Failed to resolve path to file.");
        let abs_path_file = abs_path_file.as_path();

        let abs_path_root = root
            .canonicalize()
            .expect("Failed to resolve path to root.");

        let abs_path_root = abs_path_root.as_path();

        let file_from_root = abs_path_file.strip_prefix(abs_path_root)?;
        let path_to_mutant = new_root.join(file_from_root);

        let file = File::open(&path_to_mutant)?;
        let reader = BufReader::new(file);

        // read all lines into a vector
        let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        lines[self.line_number - 1] =
            lines[self.line_number - 1].replace(&self.before, &self.after);

        let last = lines.pop().unwrap();
        lines.push(format!("{last}\n"));
        fs::write(&path_to_mutant, lines.join("\n"))
            .expect("Failed to write to file upon mutant insertion!");

        Ok(())
    }

    /// Insert the mutant in place.
    ///
    /// This will attempt to insert the mutant in the related file in the original
    /// python project (i.e. in place/where the mutant was found).
    pub fn insert(&self) -> Result<(), Box<dyn Error>> {
        let file_path = self.file_path.as_path();
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // read all lines into a vector
        let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        lines[self.line_number - 1] =
            lines[self.line_number - 1].replace(&self.before, &self.after);

        let last = lines.pop().unwrap();
        lines.push(format!("{last}\n"));
        fs::write(file_path, lines.join("\n"))
            .expect("Failed to write to file upon mutant insertion!");

        Ok(())
    }

    /// Remove the mutant.
    ///
    /// Remove a mutant from the original file after it has been inserted in place.
    /// This method is not well tested and in general the temporary directory
    /// workflow should be preferred over in place operations at the moment.
    pub fn remove(&self) -> Result<(), Box<dyn Error>> {
        let file_path = self.file_path.as_path();
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // read all lines into a vector
        let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        // revert the insert
        lines[self.line_number - 1] = self.old_line.clone();

        let last = lines.pop().unwrap();
        lines.push(format!("{last}\n"));
        fs::write(file_path, lines.join("\n"))
            .expect("Failed to write to file upon mutant removal!");

        Ok(())
    }
}

impl fmt::Display for Mutant {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "{} replaced by {} in file {} on line {}",
            self.before.green(),
            self.after.red(),
            self.file_path
                .clone()
                .into_os_string()
                .to_str()
                .expect("Failed to convert file path to string!")
                .yellow(),
            self.line_number.to_string().yellow(),
        )
    }
}

/// Search for potential mutants in a file given some replacements.
/// The replacement tuples in the Vec give the (before, after) string
/// values i.e. before can be replaced by after.
fn add_mutants_from_file(
    mutant_vec: &mut Vec<Mutant>,
    path: &PathBuf,
    replacements: &[(String, String)],
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut in_docstring = false;
    let docstring_markers = ["\"\"\"", "'''"];

    for (line_nr, line_result) in reader.lines().enumerate() {
        // ignore comments
        let line = line_result?;

        if docstring_markers
            .iter()
            .any(|&marker| line.matches(marker).count() == 2)
        {
            continue;
        }

        if docstring_markers
            .iter()
            .any(|&marker| line.contains(marker))
        {
            in_docstring = !in_docstring;
        }
        if line.starts_with('#') {
            continue;
        }

        if in_docstring {
            continue;
        }

        // also only consider stuff on left of comment
        let line_split = line.split('#').collect::<Vec<_>>()[0];
        let replacement = replacement_from_line(line_split, replacements);
        match replacement {
            Some((before, after)) => {
                let mutant = Mutant {
                    file_path: path.clone(),
                    line_number: line_nr + 1,
                    before,
                    after,
                    old_line: line,
                };
                mutant_vec.push(mutant);
            }

            None => continue,
        };
    }
    Ok(())
}

/// Remove quotes so that python strings are ignored.
fn remove_quotes(input: &str) -> String {
    let re = Regex::new(r#"'[^']*'|"[^"]*""#).unwrap();
    re.replace_all(input, "").to_string()
}

/// Find a before/after replacement tuple in `line`. Possible tuples are
/// specified in `replacements`.
///If no possible replacement is found, it returns None.
fn replacement_from_line(
    line: &str,
    replacements: &[(String, String)],
) -> Option<(String, String)> {
    let line = remove_quotes(line);

    replacements
        .iter()
        .find(|(from, _)| line.contains(from))
        .map(|(from, to)| (from.into(), to.into()))
}

/// Build a Vec of before/after replacement tuples from the specified types of
/// mutations.
fn build_replacements(mutation_types: &[MutationType]) -> Vec<(String, String)> {
    let mut replacements = Vec::new();

    let mut numbers = Vec::new();
    for n in 0..10 {
        numbers.push((n.to_string(), (n + 1).to_string()));
    }

    mutation_types
        .iter()
        .for_each(|mutation_type| match mutation_type {
            MutationType::MathOps => {
                replacements.append(&mut vec![
                    (" + ".into(), " - ".into()),
                    (" - ".into(), " + ".into()),
                    (" * ".into(), " / ".into()),
                    (" / ".into(), " * ".into()),
                ]);
            }
            MutationType::Conjunctions => {
                replacements.append(&mut vec![
                    (" and ".into(), " or ".into()),
                    (" or ".into(), " and ".into()),
                ]);
            }
            MutationType::Booleans => {
                replacements.append(&mut vec![
                    (" True ".into(), " False ".into()),
                    (" False ".into(), " True ".into()),
                ]);
            }
            MutationType::ControlFlow => {
                replacements.append(&mut vec![
                    (" else: ".into(), " elif False: ".into()),
                    (" if not ".into(), " if ".into()),
                    (" if ".into(), " if not ".into()),
                ]);
            }
            MutationType::CompOps => {
                replacements.append(&mut vec![
                    (" > ".into(), " < ".into()),
                    (" < ".into(), " > ".into()),
                    ("==".into(), "!=".into()),
                    ("!=".into(), "==".into()),
                ]);
            }
            MutationType::Numbers => replacements.append(&mut numbers),
        });

    replacements
}

#[cfg(test)]
mod tests {
    use crate::mutants::{self, build_replacements, MutationType};
    use colored::Colorize;
    use std::{
        fs::{self, read_to_string, File},
        io::Write,
    };
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_find_mutants() {
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

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_replacement_from_line_with_single_quotes() {
        let line = r#"print('a + b')"#;
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);

        let option = mutants::replacement_from_line(line, &replacements);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_replacement_from_line_with_double_quotes() {
        let line = r#"print("a + b")"#;
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);

        let option = mutants::replacement_from_line(line, &replacements);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_add_mutants_from_file() {
        let multiline_string = "def add(a, b):
    return a + b";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        write!(temp_file, "{}", multiline_string).expect("Failed to write to temporary file");

        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);

        let mut possible_mutants = Vec::<mutants::Mutant>::new();
        let _ = mutants::add_mutants_from_file(
            &mut possible_mutants,
            &temp_file.path().to_path_buf(),
            &replacements,
        );

        assert_eq!(possible_mutants.len(), 1);
        assert_eq!(possible_mutants[0].line_number, 2);
        assert_eq!(possible_mutants[0].before, String::from(" + "));
        assert_eq!(possible_mutants[0].after, String::from(" - "));
    }

    #[test]
    fn test_add_mutants_from_file_trickier() {
        let multiline_string = "def add(a, b):
    return a + b

# this is a + comment
def sub(a, b):
    return a - b

res = sub(5, 6) * add(7, 8)
print(res) # print the result *
";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        write!(temp_file, "{}", multiline_string).expect("Failed to write to temporary file");

        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);
        let mut possible_mutants = Vec::<mutants::Mutant>::new();
        let _ = mutants::add_mutants_from_file(
            &mut possible_mutants,
            &temp_file.path().to_path_buf(),
            &replacements,
        );

        assert_eq!(possible_mutants.len(), 3);

        assert_eq!(possible_mutants[0].line_number, 2);
        assert_eq!(possible_mutants[0].before, String::from(" + "));
        assert_eq!(possible_mutants[0].after, String::from(" - "));

        assert_eq!(possible_mutants[1].line_number, 6);
        assert_eq!(possible_mutants[1].before, String::from(" - "));
        assert_eq!(possible_mutants[1].after, String::from(" + "));

        assert_eq!(possible_mutants[2].line_number, 8);
        assert_eq!(possible_mutants[2].before, String::from(" * "));
        assert_eq!(possible_mutants[2].after, String::from(" / "));
    }

    #[test]
    fn test_replacement_from_line_none() {
        let line = "print('Hello World')";
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);
        let option = mutants::replacement_from_line(line, &replacements);
        println!("{:?}", option);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_replacement_from_line_math_operators() {
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);

        let line = "5 + 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" + ".into(), " - ".into()));

        let line = "5 - 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" - ".into(), " + ".into()));

        let line = "5 * 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" * ".into(), " / ".into()));

        let line = "5 / 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" / ".into(), " * ".into()));
    }

    #[test]
    fn test_replacement_from_line_conjunctions() {
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);
        let line = "True and False";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" and ".into(), " or ".into()));

        let line = "True or False";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" or ".into(), " and ".into()));
    }

    #[test]
    fn test_replacement_from_line_comparison_operators() {
        let mutation_types = vec![
            MutationType::MathOps,
            MutationType::Conjunctions,
            MutationType::Booleans,
            MutationType::ControlFlow,
            MutationType::CompOps,
            MutationType::Numbers,
        ];

        let replacements = build_replacements(&mutation_types);

        let line = "5 == 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), ("==".into(), "!=".into()));

        let line = "5 != 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), ("!=".into(), "==".into()));

        let line = "5 > 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" > ".into(), " < ".into()));

        let line = "5 < 5";
        let option = mutants::replacement_from_line(line, &replacements);
        assert_eq!(option.unwrap(), (" < ".into(), " > ".into()));
    }

    #[test]
    fn test_mutant_insert() {
        let multiline_string = "def add(a, b):
    return a + b";

        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let file_path_original = base_path.join("script.py");

        let _temp_dir_copy = tempdir().unwrap();
        let base_path_copy = temp_dir.path();
        let file_path_copy = base_path_copy.join("script.py");

        let mut file_original = File::create(&file_path_original).unwrap();
        let mut file_copy = File::create(&file_path_copy).unwrap();
        write!(file_original, "{}", multiline_string).expect("Failed to write to temporary file");
        write!(file_copy, "{}", multiline_string).expect("Failed to write to temporary file");

        let mutant = mutants::Mutant {
            file_path: file_path_original.clone(),
            line_number: 2,
            before: " + ".into(),
            after: " - ".into(),
            old_line: "    return a + b".into(),
        };

        mutant.insert().unwrap();

        let result = read_to_string(&file_path_original).unwrap();
        let desired_result = String::from("def add(a, b):\n    return a - b\n");
        assert_eq!(result, desired_result);

        mutant.remove().unwrap();

        let result = read_to_string(&file_path_original).unwrap();
        let desired_result = String::from("def add(a, b):\n    return a + b\n");
        assert_eq!(result, desired_result);

        mutant
            .insert_in_new_root(base_path, base_path_copy)
            .unwrap();
        let result = read_to_string(file_path_copy).unwrap();
        let desired_result = String::from("def add(a, b):\n    return a - b\n");
        assert_eq!(result, desired_result);

        let file_name_str = file_path_original.clone().into_os_string();
        let _file_name_str = file_name_str
            .to_str()
            .expect("Failed to convert file path to string!")
            .yellow();

        let _display = format!("{mutant}");
    }
}
