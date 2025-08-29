# Demo Analysis
A fork of megascatterbomb's demo analysis template with a simple gui and some custom algorithms.

## Usage

To build the gui, first set up a rust development environment. Then run `cargo build --release --bin gui --features gui` to compile the gui and it's optional dependencies. The executable will be in `target/release/`.

Note: The gui is very simplistic and will start parsing the demo as soon as you hit open in the file selector, so make your algorithm selection and change the parameters beforehand!

The save button saves the parameters into a json file next to the executable, and loads them from there again on the next launch.

If you want to use the command line interface instead, check the [old readme](README_old.md).