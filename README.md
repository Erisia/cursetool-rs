# cursetool-rs
Basic cursetool reimplementation in Rust, using the unofficial Curse API

 - Creates YAML files with pinned versions from a Curse manifest file
  - Generates a Nix mod description from a YAML manifest

## Getting Started (assuming Nix is installed)
If you have direnv and lorri set up, you can run `direnv allow` once to set up your environment whenever you enter the directory. Otherwise, use `nix-shell`.

## Usage

Run `cargo run -- <mode> <input> <output>`,

or `cargo build` then `target/debug/cursetool-rs <mode> <input> <output>`
```
USAGE:
    cursetool-rs <mode> <input> <output>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <mode>
            Whether to convert Curse manifest files to yaml, or yaml to nix. [possible values: curse, yaml]

    <input>
            Path to input file.
            Should be a json file in curse mode,
            and a yaml file in yaml mode
    <output>
            Path to output file.
            Will dump yaml data in curse mode,
            and nix data in yaml mode.
```
