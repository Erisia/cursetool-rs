use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use console::style;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;


use simplelog::*;

use model::*;
use options::Mode;

use crate::database::Database;
use crate::downloader::Downloader;
use crate::options::{Commandline, parse_commandline};

mod database;

mod options;
mod model;
mod downloader;


fn print_phase<T>(current: u32, total: u32, phase: T) where T: AsRef<str> {
    println!(
        "{} {}",
        style(format!("[{}/{}]", current, total)).bold().dim(),
        phase.as_ref()
    );
}

// All those 'apps littered everywhere are there to tell Rust that all of these structs live as
// long as the app does, i.e. until the end of main.
struct App<'app> {
    commandline: &'app Commandline,
    downloader: &'app Downloader<'app>,
    _database: &'app Database,
}

impl<'app> App<'app> {
    fn new(commandline: &'app Commandline, database: &'app Database, downloader: &'app Downloader<'app>) -> Self {
        App { commandline, _database: database, downloader }
    }

    fn main(&self) -> Result<()> {
        match self.commandline.mode {
            Mode::Yaml => self.generate_nix_from_yaml(&self.commandline.input_file, &self.commandline.output_file)
                .context("While generating nix from yaml")?,
            Mode::Curse => self.generate_yaml_from_curse(&self.commandline.input_file, &self.commandline.output_file)
                .context("While generating yaml from curse")?
        }
        Ok(())
    }

    fn generate_nix_from_yaml(&self, yaml_manifest_path: &Path, nix_manifest_path: &Path) -> Result<()> {
        print_phase(1, 3, "Loading manifest");
        let yaml_manifest = YamlManifest::recursive_load_from_file(yaml_manifest_path)?;
        log::info!("Found {} mods from manifest", yaml_manifest.mods.len());

        //print_phase(2, 4, format!("Fetching list of every mod for version {}", yaml_manifest.version));
        //let slug_map = self.downloader.request_mod_listing(&yaml_manifest.version)?; // map of slug -> numeric ID for every mod on Curse

        print_phase(2, 3, format!("Fetching details for {} mods", yaml_manifest.mods.len()));
        let mut mod_entries = self.generate_nix_mod_entries(yaml_manifest.mods, &yaml_manifest.version)?;
        mod_entries.sort_unstable_by_key(|m| m.slug.clone());

        print_phase(3, 3, "Writing out manifest");
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

    fn generate_nix_mod_entries(&self, mod_list: Vec<YamlMod>, version: &str) -> Result<Vec<NixMod>> {

        let progress = ProgressBar::new(mod_list.len() as u64)
            .with_style(ProgressStyle::default_bar()
                .template("{bar:30} {pos}/{len} {msg}"));
        let updater = progress.downgrade();

        mod_list.into_par_iter().progress_with(progress).map(|yaml_mod| {
            updater.upgrade().unwrap().set_message(&format!("Processing mod: {}", yaml_mod.name));

            let project_id = match yaml_mod.id {
                Some(id) => id,
                None => self.downloader.search_id_with_slug(&yaml_mod.name)?
            };
            let addon_info = self.downloader.request_addon_info(project_id)?;

            let get_all_files = |project_id: u32| -> Result<Vec<CurseModFile>> {
                self.downloader.request_mod_files(project_id, version)
                    .context(format!("Fetching files for project id {}", project_id))
            };

            let get_newest_file = |project_id: u32| -> Result<CurseModFile> {
                let mut files = get_all_files(project_id)?;
                files.sort_unstable_by_key(|f| f.file_date.clone());
                Ok(files.last().context(format!("Did not get at least one file for {:?}", yaml_mod))?.clone())
            };

            // Get a specific file if one was specified, otherwise the newest.
            let mod_file: CurseModFile = if let Some(ref file) = yaml_mod.files {
                if let Some(id) = file[0].id {
                    self.downloader.request_mod_file(project_id, id)
                        .context(format!("Looking for specific file in {:?}", yaml_mod))?
                } else {
                    get_newest_file(project_id)?
                }
            } else {
                get_newest_file(project_id)?
            };

            let CurseModFileInfo { md5, sha256, size, download_url} = self.downloader.request_mod_file_info(&mod_file.download_url)?;
            // Fix filenames and URLs
            let fixed_filename = mod_file.file_name.replace("(", "").replace(")", "");
            let fixed_src = download_url.replace("+", "%2B").replace(" ", "+");
            Ok(NixMod {
                slug: yaml_mod.name.clone(),
                title: addon_info.name,
                id: project_id,
                side: yaml_mod.side.unwrap_or(Side::Both),
                required: yaml_mod.required.unwrap_or(true),
                default: yaml_mod.default.unwrap_or(true),
                deps: vec![],
                filename: fixed_filename.clone(),
                encoded: fixed_filename,
                md5,
                sha256,
                size,
                src: fixed_src,
                page: addon_info.links.website_url,
            })
       }).collect::<Result<Vec<NixMod>, _>>()
    }

    fn generate_yaml_from_curse(&self, curse_manifest_path: &Path, yaml_manifest_path: &Path) -> Result<()> {
        log::info!("Reading manifest...");
        let manifest_file = File::open(curse_manifest_path)
            .context(format!("While opening {:?}", curse_manifest_path))?;
        let curse_manifest: CurseManifest = serde_json::from_reader(manifest_file)
            .context(format!("While parsing curse manifest YAML from {:?}", curse_manifest_path))?;
        log::info!("Found {} mods in Curse manifest", curse_manifest.files.len());
        let mut mod_entries: Vec<YamlMod> = curse_manifest.files.iter().map(|m| {
            self.generate_yaml_mod_entry(m)
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

    fn generate_yaml_mod_entry(&self, mod_info: &ModFile) -> Result<YamlMod> {
        log::info!("Fetching data for file {} in project {}", mod_info.file_id, mod_info.project_id);
        let addon_info = self.downloader.request_addon_info(mod_info.project_id)?;
        Ok(YamlMod::with_files(&addon_info.slug, mod_info.project_id, YamlModFile::with_id(mod_info.file_id)))
    }
}


fn main() -> Result<()> {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed)?;

    let api_key = std::fs::read_to_string("APIKEY")
        .context("Could not find a Curse API key!\nLogin at https://console.curseforge.com/ and save your key in a file named 'APIKEY'.")?;

    let commandline = parse_commandline();
    let database = Database::from_filesystem()?;
    let downloader = Downloader::new(&database, api_key.trim());

    let app = App::new(&commandline, &database, &downloader);

    app.main()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn with_app<F, X>(mode: Mode, input_path: PathBuf, output_path: PathBuf, f: F) -> Result<X>
        where F: FnOnce(App) -> Result<X> {
        TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed)?;

        let api_key = std::fs::read_to_string("APIKEY")
            .context("Could not find a Curse API key!\nLogin at https://console.curseforge.com/ and save your key in a file named 'APIKEY'.")?;

        let commandline = Commandline {
            mode,
            input_file: input_path,
            output_file: output_path,
        };
        let database = Database::for_tests()?;
        let downloader = Downloader::new(&database, api_key.trim());
        let app = App::new(&commandline, &database, &downloader);
        f(app)
    }

    #[test]
    fn can_generate_yaml() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let manifest_path = dir.path().join("manifest.json");
        let output_path = dir.path().join("manifest.yaml");

        write_simple_manifest(File::create(&manifest_path)?)?;

        with_app(Mode::Curse, manifest_path, output_path.clone(), |app| { app.main() })?;

        let generated_manifest: YamlManifest = serde_yaml::from_reader(&File::open(output_path)?)?;
        assert_eq!(generated_manifest.version, "1.12.2".to_string(), "Version is incorrect");
        assert_eq!(generated_manifest.mods.len(), 2, "Mod count is incorrect");
        assert_eq!(generated_manifest.imports.len(), 0, "There should be no imports");
        assert_eq!(generated_manifest.mods.get(0).unwrap().name, "iron-chests", "Iron Chests should be present");
        assert_eq!(generated_manifest.mods.get(1).unwrap().name, "jei", "JEI should be present");
        assert_eq!(generated_manifest.mods.get(0).unwrap().files.as_ref().unwrap()[0].id.unwrap(), 2747935, "File ID should be set");

        Ok(())
    }

    fn write_simple_manifest(file: File) -> Result<()> {
        serde_json::to_writer(file, &CurseManifest {
            minecraft: MinecraftVersion {
               version: "1.12.2".to_string()
            },
            files: vec![
                // JEI 4.16.1.302
                ModFile {
                    project_id: 238222,
                    file_id: 3043174,
                    required: true
                },
                // Iron Chests 7.0.72.847
                ModFile {
                    project_id: 228756,
                    file_id: 2747935,
                    required: true
                }
            ]
        })?;

        Ok(())
    }

}

