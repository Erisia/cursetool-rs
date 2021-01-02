use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::{App, Arg, crate_authors, crate_name, crate_version};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use serde_json;
use serde_yaml;
use sha2::{Digest, Sha256};
use simplelog::*;

use model::*;
use options::{Mode, Options};

mod options;
mod model;

static BASE_URL: &str = "https://addons-ecs.forgesvc.net/api/v2";

fn generate_yaml_from_curse(curse_manifest_path: &Path, yaml_manifest_path: &Path) -> Result<()> {
    log::info!("Reading manifest...");
    let curse_manifest: CurseManifest = serde_json::from_reader(File::open(curse_manifest_path)?)?;
    log::info!("Found {} mods in Curse manifest", curse_manifest.files.len());
    let mut mod_entries: Vec<YamlMod> = curse_manifest.files.iter().map(|m| {
        generate_yaml_mod_entry(m)
    }).collect::<Result<Vec<_>, _>>()?;

    mod_entries.sort_unstable_by_key(|d| d.name.clone());
    log::info!("Writing manifest...");
    serde_yaml::to_writer(&File::create(yaml_manifest_path)?,
                          &YamlManifest {
                              version: curse_manifest.minecraft.version,
                              imports: vec![],
                              mods: mod_entries,
                          })?;
    log::info!("Successfully wrote manifest!");

    Ok(())
}

fn generate_nix_from_yaml(yaml_manifest_path: &Path, nix_manifest_path: &Path) -> Result<()> {
    let yaml_manifest: YamlManifest = recursive_manifest_load(yaml_manifest_path)?;
    log::info!("Found {} mods from manifest", yaml_manifest.mods.len());
    log::info!("Fetching list of every mod for version {} from Curse...", yaml_manifest.version);
    let slug_map = request_mod_listing(&yaml_manifest.version)?; // map of slug -> numeric ID for every mod on Curse
    log::info!("Identified {} mods from Curse", slug_map.len());
    let mut mod_entries = generate_nix_mod_entries(yaml_manifest.mods, slug_map, &yaml_manifest.version)?;
    mod_entries.sort_unstable_by_key(|m| m.slug.clone());
    log::info!("Writing out manifest...");
    let formatted_mods = mod_entries.into_iter().map(|m| m.to_string()).collect::<Vec<_>>().join("\n");
    write!(BufWriter::new(File::create(nix_manifest_path)?),
           r#"{{
    "version" = "{version}";
    "imports" = [];
    "mods" = {{
    {mods}
    }};
}}"#, version = yaml_manifest.version, mods = formatted_mods)?;
    Ok(())
}

fn recursive_manifest_load(manifest_path: &Path) -> Result<YamlManifest> {
    log::info!("Reading manifest file {}...", manifest_path.display());
    let base_manifest: YamlManifest = serde_yaml::from_reader(File::open(manifest_path)?)?;
    let mut imported_manifests: Vec<YamlManifest> = Vec::new();
    for import in &base_manifest.imports {
        let relative_path = manifest_path.parent().expect("Base manifest has no parent").join(&import);
        imported_manifests.push(recursive_manifest_load(&relative_path).expect(&format!("Failed to import yaml file {}, confirm that it exists.", &import)));
    }
    Ok(base_manifest.merge(imported_manifests))
}

// Returns a mapping from mod slug to mod ID.
fn request_mod_listing(version: &str) -> Result<HashMap<String, u32>> {
    const MAX_MOD_COUNT: usize = 10000;
    let url = format!("{}/addon/search", BASE_URL);
    let client = reqwest::blocking::Client::new();
    let response: Vec<AddonInfo> = client.get(&url)
        .query(&[
            ("gameId", "432"),
            ("gameVersion", version),
            ("sort", "3"),
            ("sectionId", "6"),
            ("pageSize", &MAX_MOD_COUNT.to_string())]) // Less than 9000 1.12.2 mods as of 2021-01-01
        .send()?
        .json::<Vec<AddonInfo>>()?;
    if response.len() >= MAX_MOD_COUNT {
        bail!("The first page of results is full, some mods may not be present in list.");
    }
    let mut result = HashMap::new();
    for addon in response {
        let slug = get_slug_from_webpage_url(&addon.website_url)
            .context(format!("Fetching slug for {:?}", addon))?;
        result.insert(slug, addon.id);
    }
    Ok(result)
}

fn generate_nix_mod_entries(mod_list: Vec<YamlMod>, slug_map: HashMap<String, u32>, version: &str) -> Result<Vec<NixMod>> {
    mod_list.into_iter().map(|yaml_mod: YamlMod| {
        log::info!("Processing mod: {}", yaml_mod.name);
        let project_id = match yaml_mod.id {
            Some(id) => id,
            None     => *slug_map.get(&yaml_mod.name).context(format!("Unable to find the Curse ID for mod {}. If the mod name is correct, try specifying the ID manually.", yaml_mod.name))?
        };
        let addon_info = request_addon_info(project_id)?;

        fn get_all_files(project_id: u32) -> Result<impl Iterator<Item=CurseModFile>> {
            Ok(
                request_mod_files(project_id)
                    .context(format!("Fetching files for project id {}", project_id))?
                    .into_iter()
            )
        }

        let get_newest_file = |project_id: u32| -> Result<CurseModFile> {
            // Filter out only those files which match the game version.
            let mut files = get_all_files(project_id)?
                .filter(|f| f.game_version.iter().any(|v| v == version))
                .collect::<Vec<CurseModFile>>();
            files.sort_unstable_by_key(|f| f.file_date.clone());
            Ok(files.last().context(format!("Did not get at least one file for {:?}", yaml_mod))?.clone())
        };

        // Get a specific file if one was specified, otherwise the newest.
        let mod_file: CurseModFile = if let Some(ref file) = yaml_mod.files {
            if let Some(id) = file[0].id {
                get_all_files(project_id)?
                    .find(|&ref f| f.id == id)
                    .context(format!("Looking for specific file in {:?}", yaml_mod))?
            } else {
                get_newest_file(project_id)?
            }
        } else {
            get_newest_file(project_id)?
        };

        let (md5, sha256, size, download_url) = get_mod_info(&mod_file.download_url)?;

        Ok(NixMod {
            slug: yaml_mod.name.clone(),
            title: addon_info.name,
            id: project_id,
            side: yaml_mod.side.unwrap_or(Side::Both),
            required: yaml_mod.required.unwrap_or(true),
            default: yaml_mod.default.unwrap_or(true),
            deps: vec![],
            filename: mod_file.clone().file_name,
            encoded: mod_file.file_name,
            md5,
            sha256,
            size,
            src: download_url,
            page: addon_info.website_url,
        })
    }).collect::<Result<Vec<NixMod>, _>>()
}

fn request_mod_files(project_id: u32) -> Result<Vec<CurseModFile>> {
    let url = format!("{}/addon/{}/files", BASE_URL, project_id);
    Ok(reqwest::blocking::get(&url)
        .context(format!("Fetching files for project id {}", project_id))?
        .json::<Vec<CurseModFile>>()
        .context("Parsing files list as JSON")?)
}

fn get_mod_info(url: &str) -> Result<(String, String, u64, String)> {
    let redirected_url = url.replace("edge.forgecdn.net", "media.forgecdn.net");
    let mut buf: Vec<u8> = vec![];
    let size = reqwest::blocking::get(&redirected_url)?.copy_to(&mut buf)?;
    let md5 = format!("{:x}", md5::compute(&buf));
    let sha256 = format!("{:x}", Sha256::digest(&buf));

    Ok((md5, sha256, size, redirected_url))
}

fn generate_yaml_mod_entry(mod_info: &ModFile) -> Result<YamlMod> {
    log::info!("Fetching data for file {} in project {}", mod_info.file_id, mod_info.project_id);
    let addon_info = request_addon_info(mod_info.project_id)?;
    let mod_slug = get_slug_from_webpage_url(&addon_info.website_url)?;
    Ok(YamlMod::with_files(&mod_slug, mod_info.project_id, YamlModFile::with_id(mod_info.file_id)))
}

fn request_addon_info(project_id: u32) -> Result<AddonInfo> {
    let url = format!("{}/addon/{}", BASE_URL, project_id);
    Ok(
        reqwest::blocking::get(&url)
            .context(format!("Fetching addon info for project id {}", project_id))?
            .json()
            .context("Parsing addon info as JSON")?
    )
}

fn get_slug_from_webpage_url(url: &str) -> Result<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r".*/(?P<slug>.*)$").unwrap();
    }
    Ok(
        RE.captures(url)
            .and_then(|c| c.name("slug"))
            .context(format!("Extracting slug from {}", url))?
            .as_str().into()
    )
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
        Mode::FromYaml => generate_nix_from_yaml(&options.input_file, &options.output_file).unwrap(),
        Mode::FromCurse => generate_yaml_from_curse(&options.input_file, &options.output_file).unwrap()
    }
}
