#![feature(try_trait)]
use reqwest;
use clap::{App, Arg, crate_name, crate_authors, crate_version};
use std::path::Path;
mod options;
use options::{Options, Mode};
mod manifest_error;
use manifest_error::ManifestError;
extern crate simplelog;
use simplelog::*;
use serde_json;
use serde_yaml;
use serde::{Serialize, Deserialize};
use std::fs::File;
extern crate regex;
use regex::Regex;
#[macro_use] extern crate lazy_static;

static BASE_URL: &str = "https://addons-ecs.forgesvc.net/api/v2";

#[derive(Serialize, Deserialize)]
struct MinecraftVersion {
    version: String
}
#[derive(Serialize, Deserialize)]
struct ModFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    required: bool
}
#[derive(Serialize, Deserialize)]
struct CurseManifest {
    minecraft: MinecraftVersion,
    files: Vec<ModFile>
}
#[derive(Serialize, Deserialize)]
struct AddonInfo {
    name: String,
    #[serde(rename = "websiteUrl")]
    website_url: String
}

#[derive(Serialize, Deserialize)]
struct YamlModFile {
    #[serde(skip_serializing_if="Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    id: Option<u32>,
    #[serde(skip_serializing_if="Option::is_none")]
    maturity: Option<String>,
    #[serde(rename = "filePageUrl")]
    #[serde(skip_serializing_if="Option::is_none")]
    file_page_url: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    src: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    md5: Option<String>
}

#[derive(Serialize, Deserialize)]
struct YamlMod {
    name: String,
    #[serde(skip_serializing_if="Option::is_none")]
    side: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    default: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    files: Option<Vec<YamlModFile>>
}

#[derive(Serialize, Deserialize)]
struct YamlManifest {
    version: String,
    imports: Vec<String>,
    mods: Vec<YamlMod>
}

impl YamlModFile {
    fn with_id(id: u32) -> YamlModFile {
        YamlModFile {
            name: None,
            id: Some(id),
            maturity: None,
            file_page_url: None,
            src: None,
            md5: None
        }
    }
}

impl YamlMod {
    fn with_files(name: String, file: YamlModFile) -> YamlMod {
        YamlMod {
            name: name,
            side: None,
            required: None,
            default: None,
            files: Some(vec![file])
        }
    }
}

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
    log::debug!("Fetching data for file {} in project {}", mod_info.file_id, mod_info.project_id);
    let addon_info = request_addon_info(mod_info.project_id)?;
    let mod_slug = get_slug_from_webpage_url(&addon_info.website_url)?;
    Ok(YamlMod::with_files(mod_slug, YamlModFile::with_id(mod_info.file_id)))
}

fn request_addon_info(project_id: u32) -> Result<AddonInfo, ManifestError> {
    let url = format!("{}/addon/{}", BASE_URL, project_id);
    //let url = reqwest::Url::parse(BASE_URL)?.join("addon")?.join(&project_id.to_string())?;
    Ok(reqwest::blocking::get(&url)?.json::<AddonInfo>()?)
}

fn get_slug_from_webpage_url(url: &str) -> Result<String, ManifestError> {
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
