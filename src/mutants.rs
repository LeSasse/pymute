use glob::glob;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

pub fn find_mutants(glob_expression: &str) -> Vec<Mutant> {
    let mut possible_mutants = Vec::<Mutant>::new();

    for entry in glob(glob_expression).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if file_name.starts_with("test_") {
                    continue;
                }
                if file_name.ends_with("_test.py") {
                    continue;
                }
                add_mutants_from_file(&mut possible_mutants, &path);
            }
            Err(e) => println!("{}", e),
        }
    }

    possible_mutants
}

#[derive(Debug)]
pub struct Mutant {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub before: String,
    pub after: String,
}

impl Mutant {
    pub fn insert(&self, root: &Path, new_root: &Path) {
        let file_from_root = self.file_path.strip_prefix(&root).unwrap();
        let path_to_mutant = new_root.join(&file_from_root);

        let file = File::open(&path_to_mutant).unwrap();
        let reader = BufReader::new(file);

        // read all lines into a vector
        let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
        lines[self.line_number - 1] =
            lines[self.line_number - 1].replace(&self.before, &self.after);

        fs::write(&path_to_mutant, lines.join("\n")).expect("");
    }
}

fn replacement_from_line(line: &str) -> Option<(String, String)> {
    match line {
        _l if line.contains("+") => Some(("+".into(), "-".into())),
        _l if line.contains("-") => Some(("-".into(), "+".into())),
        _l if line.contains("*") => Some(("*".into(), "/".into())),
        _l if line.contains("/") => Some(("/".into(), "*".into())),
        _l if line.contains(" and ") => Some((" and ".into(), " or ".into())),
        _l if line.contains(" or ") => Some((" or ".into(), " and ".into())),
        _ => None,
    }
}

fn add_mutants_from_file(mutant_vec: &mut Vec<Mutant>, path: &PathBuf) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for (line_nr, line_result) in reader.lines().enumerate() {
        // ignore comments
        let line = line_result.unwrap();
        if line.starts_with("#") {
            continue;
        }

        // also only consider stuff on left of comment
        let line = line.split("#").collect::<Vec<_>>()[0];
        let replacement = replacement_from_line(line);
        match replacement {
            Some((before, after)) => {
                let mutant = Mutant {
                    file_path: path.clone(),
                    line_number: line_nr + 1,
                    before: before,
                    after: after,
                };
                mutant_vec.push(mutant);
            }

            None => continue,
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::mutants;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_add_mutants_from_file() {
        let multiline_string = "def add(a, b):
    return a + b";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        write!(temp_file, "{}", multiline_string).expect("Failed to write to temporary file");

        let mut possible_mutants = Vec::<mutants::Mutant>::new();
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
}
