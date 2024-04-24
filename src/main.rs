use pymute::mutants::find_mutants;

pub mod cli;

fn main() {
    find_mutants("../pytest-demo/**/*.py")
}
