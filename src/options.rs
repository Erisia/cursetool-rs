use std::path::PathBuf;

use structopt::StructOpt;
use structopt::clap::arg_enum;

#[derive(Debug, StructOpt)]
#[structopt(about = "Rust implementation of Cursetool")]
pub struct Commandline {
    #[structopt(help = "Whether to convert Curse manifest files to yaml, or yaml to nix.")]
    pub mode: Mode,
    #[structopt(help = "Path to input file.\n\
                    Should be a json file in curse mode,\n\
                    and a yaml file in yaml mode")]
    pub input_file: PathBuf,
    #[structopt(help = "Path to output file.\n\
                    Will dump yaml data in curse mode,\n\
                    and nix data in yaml mode.")]
    pub output_file: PathBuf,
}

arg_enum! {
    #[derive(Debug)]
    pub enum Mode {
        Curse,
        Yaml,
    }
}

pub fn parse_commandline() -> Commandline {
    Commandline::from_args()
}