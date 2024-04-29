use colored::Colorize;
use glob::glob;
use regex::Regex;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Find potential python mutants from files that match the glob expression.
///
/// It will ignore any files that start with test_* and that end with *_test.py
/// to avoid mutating pytest tests.
///
/// Parameters
/// ----------
/// glob_expression: &str compatible with the `glob` crate.
pub fn find_mutants(glob_expression: &str) -> Result<Vec<Mutant>, Box<dyn Error>> {
    let mut possible_mutants = Vec::<Mutant>::new();

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
                let _ = add_mutants_from_file(&mut possible_mutants, &path);
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

fn add_mutants_from_file(
    mutant_vec: &mut Vec<Mutant>,
    path: &PathBuf,
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
        let replacement = replacement_from_line(line_split);
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

fn remove_quotes(input: &str) -> String {
    let re = Regex::new(r#"'[^']*'|"[^"]*""#).unwrap();
    re.replace_all(input, "").to_string()
}

fn replacement_from_line(line: &str) -> Option<(String, String)> {
    let line = remove_quotes(line);
    let replacements = vec![
        // mathematical operators
        (" + ", " - "),
        (" - ", " + "),
        (" * ", " / "),
        (" / ", " * "),
        // conjunctions
        (" and ", " or "),
        (" or ", " and "),
        // booleans
        (" True ", " False "),
        (" False ", " True "),
        // control flow
        (" else: ", " elif False: "),
        (" if not ", " if "),
        (" if ", " if not "),
        // comparisons
        (" > ", " < "),
        (" < ", " > "),
        ("==", "!="),
        ("!=", "=="),
        // numpy/pandas shenanigans
        ("std(", "mean("),
        ("mean(", "std("),
        // other built-ins
        ("range(", "list("),
    ];

    replacements
        .iter()
        .find(|(from, _)| line.contains(from))
        .map(|&(from, to)| (from.into(), to.into()))
}

#[cfg(test)]
mod tests {
    use crate::mutants;
    use std::{
        fs::{self, File},
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

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_replacement_from_line_with_single_quotes() {
        let line = r#"print('a + b')"#;
        let option = mutants::replacement_from_line(&line);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_replacement_from_line_with_double_quotes() {
        let line = r#"print("a + b")"#;
        let option = mutants::replacement_from_line(&line);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_add_mutants_from_file() {
        let multiline_string = "def add(a, b):
    return a + b";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        write!(temp_file, "{}", multiline_string).expect("Failed to write to temporary file");

        let mut possible_mutants = Vec::<mutants::Mutant>::new();
        let _ =
            mutants::add_mutants_from_file(&mut possible_mutants, &temp_file.path().to_path_buf());

        assert_eq!(possible_mutants.len(), 1);
        assert_eq!(possible_mutants[0].line_number, 2);
        assert_eq!(possible_mutants[0].before, String::from("+"));
        assert_eq!(possible_mutants[0].after, String::from("-"));
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

        let mut possible_mutants = Vec::<mutants::Mutant>::new();
        let _ =
            mutants::add_mutants_from_file(&mut possible_mutants, &temp_file.path().to_path_buf());

        assert_eq!(possible_mutants.len(), 3);

        assert_eq!(possible_mutants[0].line_number, 2);
        assert_eq!(possible_mutants[0].before, String::from("+"));
        assert_eq!(possible_mutants[0].after, String::from("-"));

        assert_eq!(possible_mutants[1].line_number, 6);
        assert_eq!(possible_mutants[1].before, String::from("-"));
        assert_eq!(possible_mutants[1].after, String::from("+"));

        assert_eq!(possible_mutants[2].line_number, 8);
        assert_eq!(possible_mutants[2].before, String::from("*"));
        assert_eq!(possible_mutants[2].after, String::from("/"));
    }

    #[test]
    fn test_replacement_from_line_none() {
        let line = "print('Hello World')";
        let option = mutants::replacement_from_line(&line);
        println!("{:?}", option);
        assert!(option.is_none(), "Expected the option to be None");
    }

    #[test]
    fn test_replacement_from_line_math_operators() {
        let line = "5 + 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("+".into(), "-".into()));

        let line = "5 - 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("-".into(), "+".into()));

        let line = "5 * 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("*".into(), "/".into()));

        let line = "5 / 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("/".into(), "*".into()));
    }

    #[test]
    fn test_replacement_from_line_conjunctions() {
        let line = "True and False";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), (" and ".into(), " or ".into()));

        let line = "True or False";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), (" or ".into(), " and ".into()));
    }

    #[test]
    fn test_replacement_from_line_comparison_operators() {
        let line = "5 == 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("==".into(), "!=".into()));

        let line = "5 != 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("!=".into(), "==".into()));

        let line = "5 > 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), (">".into(), "<".into()));

        let line = "5 < 5";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), ("<".into(), ">".into()));
    }
}
