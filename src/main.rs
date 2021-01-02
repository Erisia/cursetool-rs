#![feature(try_trait)]
use reqwest;
use clap::{App, Arg, crate_name, crate_authors, crate_version};
use std::path::Path;
mod options;
use options::{Options, Mode};
mod manifest_error;
use manifest_error::ManifestError;
mod model;
use model::*;
extern crate simplelog;
use simplelog::*;
use serde_json;
use serde_yaml;
use std::fs::File;
extern crate regex;
use regex::Regex;
#[macro_use] extern crate lazy_static;

static BASE_URL: &str = "https://addons-ecs.forgesvc.net/api/v2";

fn generate_yaml_from_curse(curse_manifest_path: &Path, yaml_manifest_path: &Path) -> Result<(), ManifestError> { 
    log::info!("Reading manifest...");
    let curse_manifest: CurseManifest = serde_json::from_reader(File::open(curse_manifest_path)?)?;
    log::info!("Found {} mods in Curse manifest", curse_manifest.files.len());
    let mut mod_entries: Vec<YamlMod> = curse_manifest.files.iter().map(|m| {
        generate_yaml_mod_entry(m)
    }).collect::<Result<Vec<_>,_>>()?;

    mod_entries.sort_unstable_by_key(|d| d.name.clone());
    log::info!("Writing manifest...");
    serde_yaml::to_writer(&File::create(yaml_manifest_path)?,
    &YamlManifest {
        version: curse_manifest.minecraft.version,
        imports: vec![],
        mods: mod_entries
    })?;
    log::info!("Successfully wrote manifest!");

    Ok(())
}

fn generate_yaml_mod_entry(mod_info: &ModFile) -> Result<YamlMod, ManifestError> {
    log::info!("Fetching data for file {} in project {}", mod_info.file_id, mod_info.project_id);
    let addon_info = request_addon_info(mod_info.project_id)?;
    let mod_slug = get_slug_from_webpage_url(&addon_info.website_url)?;
    Ok(YamlMod::with_files(&mod_slug, YamlModFile::with_id(mod_info.file_id)))
}

fn request_addon_info(project_id: u32) -> Result<AddonInfo, reqwest::Error> {
    let url = format!("{}/addon/{}", BASE_URL, project_id);
    //let url = reqwest::Url::parse(BASE_URL)?.join("addon")?.join(&project_id.to_string())?;
    reqwest::blocking::get(&url)?.json::<AddonInfo>()
}

fn request_download_url(mod_info: &ModFile) -> Result<String, reqwest::Error> {
    let url = format!("{}/addon/{}/file/{}/download-url", BASE_URL, mod_info.project_id, mod_info.file_id);
    reqwest::blocking::get(&url)?.text()
}

fn get_slug_from_webpage_url(url: &str) -> Result<String, std::option::NoneError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r".*/(?P<slug>.*)$").unwrap();
    }
    Ok(RE.captures(url)?.name("slug")?.as_str().to_owned())
}

fn main() {
    TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed).unwrap();
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
