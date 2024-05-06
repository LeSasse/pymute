# Version `0.2.1`

* Add a [ctrlc](https://crates.io/crates/ctrlc) as a dependency to better handle SIGINT in `runner.rs` and
reliably clean up the temporary directories
* Improve error handling to propagate errors to `main.rs` and exit with appropriate
exit codes and (hopefully) meaningful error messages
* Introduce [cargo nextest](https://crates.io/crates/cargo-nextest) to run tests with cargo nextest:
  - `cargo test` fails using MultipleHandelers, since threads are not independent
  and are running `run_mutants` at the same time (see similar issue here https://github.com/jdx/mise/issues/570)
  - `cargo test` is still used in the CI to run doctests since `nextest` does not seem to be able to do it
