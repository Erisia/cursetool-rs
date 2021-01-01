use reqwest;
use clap::{App, Arg, crate_name, crate_authors, crate_version};
use std::path::Path;
mod options;
use options::{Options, Mode};
extern crate simplelog;
use simplelog::*;
use serde_json;
use serde::{Serialize, Deserialize};
use std::fs::File;

#[derive(Serialize, Deserialize)]
struct MinecraftVersion {
    version: String
}
#[derive(Serialize, Deserialize)]
struct ModFile {
    projectID: u32,
    fileID: u32,
    required: bool
}
#[derive(Serialize, Deserialize)]
struct CurseManifest {
    minecraft: MinecraftVersion,
    files: Vec<ModFile>
}

fn generate_yaml_from_curse(curse_manifest_path: &Path, yaml_manifest_path: &Path) -> Result<(), std::io::Error> { 
    log::info!("Reading manifest...");
    let curse_manifest: CurseManifest = serde_json::from_reader(File::open(curse_manifest_path)?)?;
    log::info!("Found {} mods in Curse manifest", curse_manifest.files.len());

    Ok(())
}

fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap();
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::with_name("mode")
             .required(true)
             .possible_values(&["curse", "yaml"])
             .takes_value(true)
             .help("Whether to convert Curse manifest files to yaml, or yaml to nix.")
             .next_line_help(true))
        .arg(Arg::with_name("input")
             .required(true)
             .takes_value(true)
             .help("Path to input file.\n\
                    Should be a json file in curse mode,\n\
                    and a yaml file in yaml mode")
             .next_line_help(true))
        .arg(Arg::with_name("output")
             .required(true)
             .takes_value(true)
             .help("Path to output file.\n\
                    Will dump yaml data in curse mode,\n\
                    and nix data in yaml mode.")
             .next_line_help(true))
        .get_matches();
    
    let options = Options::from_clap(&matches);

    match options.mode {
        // Full cursetool implementation pending
        Mode::FromYaml => panic!("Processing yaml files is not yet implemented!"),
        Mode::FromCurse => generate_yaml_from_curse(&options.input_file, &options.output_file).unwrap()
    }
}
