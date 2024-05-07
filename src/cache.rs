use crate::mutants::Mutant;
use std::{error::Error, fs::File, path::PathBuf};

pub fn write_csv_cache(mutants: &[Mutant], cache_path: &PathBuf) -> Result<String, Box<dyn Error>> {
    let file = File::create(cache_path)?;
    let mut wtr = csv::Writer::from_writer(file);

    for mutant in mutants.iter() {
        wtr.serialize(mutant)?;
    }

    Ok("Results written to cache".to_string())
}

pub fn read_csv_cache(cache_path: &PathBuf) -> Result<Vec<Mutant>, Box<dyn Error>> {
    let file = File::open(cache_path)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut mutants = Vec::new();
    for mutant in reader.deserialize() {
        let mutant: Mutant = mutant.unwrap();
        mutants.push(mutant);
    }

    Ok(mutants)
}

#[cfg(test)]
mod tests {
    use crate::cache::{read_csv_cache, write_csv_cache};
    use crate::mutants;
    use std::{
        fs::{read_to_string, File},
        io::Write,
        path::PathBuf,
    };
    use tempfile::tempdir;

    #[test]
    fn test_write_csv_cache() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let file_path_cache = base_path.join("cache.csv");

        // don't use new here so we can use an unreal path
        let mutant_one = mutants::Mutant {
            file_path: PathBuf::from("/projects/project/script.py"),
            line_number: 2,
            before: " + ".into(),
            after: " - ".into(),
            status: mutants::MutantStatus::NotRun,
        };

        let mutant_two = mutants::Mutant {
            file_path: PathBuf::from("/projects/project/script.py"),
            line_number: 65,
            before: " - ".into(),
            after: " + ".into(),
            status: mutants::MutantStatus::NotRun,
        };

        let mutants = vec![mutant_one, mutant_two];

        write_csv_cache(&mutants, &file_path_cache).unwrap();

        let result = read_to_string(&file_path_cache).unwrap();
        let expected_string = r#"file_path,line_number,before,after,status
/projects/project/script.py,2, + , - ,NotRun
/projects/project/script.py,65, - , + ,NotRun
"#
        .to_string();

        assert_eq!(expected_string, result);
    }

    #[test]
    fn test_read_csv_cache() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();
        let file_path_cache = base_path.join("cache.csv");

        let serialised = r#"file_path,line_number,before,after,status
/projects/project/script.py,2, + , - ,NotRun
/projects/project/script.py,65, - , + ,NotRun
"#;

        // create inner scope to make sure the file handle is out of scope later
        {
            let mut file_cache = File::create(&file_path_cache).unwrap();
            write!(file_cache, "{}", serialised).expect("Failed to write to temporary file");
        }
        let mutants_cached = read_csv_cache(&file_path_cache).unwrap();

        // don't use new here so we can use an unreal path
        let mutant_one = mutants::Mutant {
            file_path: PathBuf::from("/projects/project/script.py"),
            line_number: 2,
            before: " + ".into(),
            after: " - ".into(),
            status: mutants::MutantStatus::NotRun,
        };

        let mutant_two = mutants::Mutant {
            file_path: PathBuf::from("/projects/project/script.py"),
            line_number: 65,
            before: " - ".into(),
            after: " + ".into(),
            status: mutants::MutantStatus::NotRun,
        };

        assert_eq!(mutant_one, mutants_cached[0]);
        assert_eq!(mutant_two, mutants_cached[1]);
    }
}
