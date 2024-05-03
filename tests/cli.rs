use assert_cmd::prelude::*;
use std::process::Command;

use std::{fs::File, io::Write};
use tempfile::tempdir;

#[test]
fn test_pymute_command() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut cmd = Command::cargo_bin("pymute")?;

    cmd.arg(base_path.to_str().unwrap());
    cmd.assert().success();

    // best be safe and close it
    temp_dir.close().unwrap();
    Ok(())
}
