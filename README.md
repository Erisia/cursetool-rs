# cursetool-rs
Basic cursetool reimplementation in Rust, using the unofficial Curse API

 - Creates YAML files with pinned versions from a Curse manifest file
  - Generates a Nix mod Description from a YAML manifest

## Getting Started (assuming Nix is installed)
If you have direnv and lorri set up, you can run `direnv allow` once to set up your environment whenever you enter the directory. Otherwise, use `nix-shell`.

You need to be running Nix with flakes enabled.

## Development

Run `nix develop`, then use Rust / cargo as normal.

Use `nix flake update` to update non-Rust dependencies, and `cargo update` for Rust dependencies.
You will need to update the cargoSha256 in flake.nix after doing the latter.

## Usage

Run `nix run <path-to-this-dir> <mode> <input> <output>`

E.g, `nix run cursetool-rs yaml manifest/e30.yml manifest/e30.nix`.

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
