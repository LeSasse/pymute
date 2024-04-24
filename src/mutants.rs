use glob::glob;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

fn replacement_from_line(line: &str) -> Option<(String, String)> {
    match line {
        l if line.contains("+") => Some(("+".into(), "-".into())),
        l if line.contains("-") => Some(("-".into(), "+".into())),
        l if line.contains("*") => Some(("*".into(), "/".into())),
        l if line.contains("/") => Some(("/".into(), "*".into())),
        l if line.contains(" and ") => Some((" and ".into(), " or ".into())),
        l if line.contains(" or ") => Some((" or ".into(), " and ".into())),
        _ => None,
    }
}

fn add_mutants_from_file(mut mutant_vec: &Vec<Mutant>, path: &PathBuf) {
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
                println!("{:?}", mutant);
            }

            None => continue,
        };
    }
}

pub fn find_mutants(glob_expression: &str) {
    let mut possible_mutants = Vec::<Mutant>::new();

    for entry in glob(glob_expression).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let path_str = path.clone().into_os_string().into_string().unwrap();
                if path_str.contains("test") {
                    continue;
                }
                add_mutants_from_file(&mut possible_mutants, &path);
            }
            Err(e) => println!("{}", e),
        }
    }
}

#[derive(Debug)]
pub struct Mutant {
    file_path: PathBuf,
    line_number: usize,
    before: String,
    after: String,
}

#[cfg(test)]
mod tests {
    use crate::mutants;

    #[test]
    fn test_replacement_from_line() {
        let line = "print('Hello World')";
        let option = mutants::replacement_from_line(&line);
        println!("{:?}", option);
        assert!(option.is_none(), "Expected the option to be None");

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

        let line = "True and False";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), (" and ".into(), " or ".into()));

        let line = "True or False";
        let option = mutants::replacement_from_line(&line);
        assert_eq!(option.unwrap(), (" or ".into(), " and ".into()));
    }
}
