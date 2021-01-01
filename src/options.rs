use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum Mode {
    FromCurse,
    FromYaml
}

#[derive(Debug, Clone)]
pub(crate) struct Options {
    pub mode: Mode,
    pub input_file: PathBuf,
    pub output_file: PathBuf
}

impl Options {
    pub fn from_clap(matches: &clap::ArgMatches<'_>) -> Options {
        Options {
            mode: match matches.value_of("mode").unwrap() {
                "curse" => Mode::FromCurse,
                "yaml" => Mode::FromYaml,
                _ => Mode::FromCurse
            },
            input_file: PathBuf::from(matches.value_of_os("input").unwrap()),
            output_file: PathBuf::from(matches.value_of_os("output").unwrap())
        }
    }
}
