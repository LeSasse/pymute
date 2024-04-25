use crate::mutants::Mutant;
use cp_r::CopyOptions;
use rayon::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn run_mutant(mutant: &Mutant, root: &PathBuf, tests_glob: &String) {
    let dir = tempdir().unwrap();
    let root_path = root.as_path();

    let stats = CopyOptions::new().copy_tree(root_path, dir.path()).unwrap();

    mutant.insert(&root_path, dir.path());

    let output = Command::new("pytest")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .arg("-x")
        .arg(tests_glob)
        .current_dir(&dir)
        .status();

    if let Ok(status) = output {
        if status.success() {
            println!(
                "[MISSED]: {} replaced by {} in file {:?} on line {}",
                mutant.before, mutant.after, mutant.file_path, mutant.line_number
            )
        }
    }
}

pub fn pytest_mutants(mutants: &Vec<Mutant>, root: &PathBuf, tests_glob: &String) {
    mutants.par_iter().for_each(|mutant| {
        run_mutant(&mutant, &root, tests_glob);
    })
}
