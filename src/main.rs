use clap::Parser;
use colored::Colorize;
use pymute::{run, Arguments};
use std::process;

fn main() {
    let args = Arguments::parse();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
        .expect("Failed to set the number of threads using rayon.");

    match run(&args) {
        Ok(_) => println!("{}!", "Success".green()),
        Err(err) => {
            println!("{}: {}", "Error".red(), err);
            process::exit(1);
        }
    };
}
