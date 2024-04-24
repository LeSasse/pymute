use glob::glob;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

fn replacement_from_line(line: &str) -> Option<(String, String)> {
    match line {
        l if line.contains("+") => Some(("+".into(), "-".into())),
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
                    line_number: line_nr,
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
