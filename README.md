# Pymute: A Mutation Testing Tool for Python/Pytest written in Rust

Pymute is inspired mainly by my experience of using [cargo mutants](https://mutants.rs/).
I used it in a rust project and was blown away by it, so had to search for something
similar for python/pytest projects. Quickly found [mut.py](https://github.com/mutpy/mutpy)
and [the pynguin fork of mut.py](https://github.com/se2p/mutpy-pynguin) among
some other solutions, but none of them seemed quite stable across different python
versions as they rely on the Python AST which (I suppose).

Pymute takes a somewhat naive approach and simply creates a temporary directory
for every mutant, and then runs pytest in that working directory independent
of other mutations. Mutations are inserted by simply manipulating the text in the
*.py files rather than operating on the AST and therefore should work across most versions.

## Installation

To install `pymute` make sure you have rust and cargo installed on your system
(check `cargo --version`). You can follow instructions here to install rust and
its toolchain: https://www.rust-lang.org/tools/install

You can install `pymute` via cargo from GitHub as:
```
cargo install --git https://github.com/LeSasse/pymute.git
```

## How to run it:

Pymute assumes that your pytest tests can be run from the root directory of your
python project and that they can be run in independent copies of your python project
without failing. For example if you have a project in:

```
~/projects/my_project
```
and you copy this as:
```
cp -r ~/projects/my_project /tmp/my_project
```

Then you should be able to run the tests in the copy as:
```
cd /tmp/my_project
pytest .
```

### Example

This repository comes with a small example of a python project with some basic
tests. You can test this out by cloning this repo if you like:

```
git clone https://github.com/LeSasse/pymute.git
cd pymute
pymute example
```
This should give you the following output:

```
[MISSED] Mutant Survived: + replaced by - in file example/src/model.py on line 6
[MISSED] Mutant Survived: == replaced by != in file example/src/model.py on line 27
```

This means that the tests provided in the example "missed" these two mutations.
That is:
1. + was replaced with - in file example/src/model.py on line 6
2. == replaced by != in file example/src/model.py on line 27

These replacements change the behaviour of the program, i.e. represent a bug/regression,
but the tests passed anyways. In other words, the tests were unable to safeguard 
the project from these bugs and they could have been checked into the main branch.

### On a Bigger Project

Running long test suites for potentially hundreds of mutants may not be feasible
with the approach that pymute takes. Therefore pymute provides the options to 
mutate only subsets of your program and to run only subsets of your tests.

We can use a bigger project like [junifer](https://github.com/juaml/junifer) as an example:

```
git clone https://github.com/juaml/junifer.git --recurse-submodules
cd junifer
```

We could now run `pymute` on the whole project as:

```
pymute . --tests junifer/
```
where `"."` is the root directory of the python project I want to test and
the `--tests` option is the path that will be passed on to pytest to tell
it where to find and run tests. However, this approach will likely take
a lot of time, because there will be a lot of mutants and a long test suite to run.

Instead, maybe we only want to try and improve the tests in junifers datagrabber
sub-directory, so we could tell pymute to only mutate code in there and to only 
run tests that are specifically designed for the code in that sub-directory:

```
pymute . --modules "junifer/datagrabber/**/*.py" --tests junifer/datagrabber/tests/
```
You can see that modules takes a glob expression and it will mutate the matched
files. It is important to wrap the glob expression in quotes.
Importantly, it does filter out files that start with `"test_"` and files
that end with `*_test.py` so as to not create mutations for pytest tests.
The `--tests` option tells pymute to run only tests in `junifer/datagrabber/tests/`.


```toml
[testenv:test_datagrabber]
skip_install = false
passenv =
    HOME
deps =
    pytest
commands =
    pytest junifer/datagrabber/tests/
```
