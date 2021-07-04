use serde::{Serialize, Deserialize};
use serde_json::json;
use std::collections::{HashSet, HashMap};
use std::path::Path;
use anyhow::{Result, Context};
use std::fs::File;

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftVersion {
    pub version: String
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ModFile {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    pub required: bool
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CurseManifest {
    pub minecraft: MinecraftVersion,
    pub files: Vec<ModFile>
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AddonInfo {
    pub name: String,
    #[serde(rename = "websiteUrl")]
    pub website_url: String,
    pub id: u32
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YamlModFile {
    #[serde(skip_serializing_if="Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub maturity: Option<String>,
    #[serde(rename = "filePageUrl")]
    #[serde(skip_serializing_if="Option::is_none")]
    pub file_page_url: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub md5: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YamlMod {
    pub name: String,
    pub id: u32,
    #[serde(skip_serializing_if="Option::is_none")]
    pub side: Option<Side>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub files: Option<Vec<YamlModFile>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YamlManifest {
    pub version: String,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub mods: Vec<YamlMod>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CurseModFileInfo {
    pub md5: String,
    pub sha256: String,
    pub size: u64,
    pub download_url: String,
}

impl YamlManifest {
    pub(crate) fn recursive_load_from_file(manifest_path: &Path) -> Result<Self> {
        log::info!("Reading manifest file {}...", manifest_path.display());
        let manifest_file = File::open(manifest_path)
            .context(format!("While opening {:?}", manifest_path))?;
        let base_manifest: YamlManifest = serde_yaml::from_reader(manifest_file)
            .context(format!("While parsing YAML from {:?}", manifest_path))?;

        let mut imported_manifests: Vec<YamlManifest> = Vec::new();
        for import in &base_manifest.imports {
            let relative_path = manifest_path.parent().expect("Base manifest has no parent").join(&import);
            imported_manifests.push(Self::recursive_load_from_file(&relative_path)
                .context(format!("While importing yaml file {}", import))?);
        }
        Ok(base_manifest.merge(imported_manifests))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Client,
    Server,
    Both
}

pub struct NixMod {
    pub slug: String,
    pub title: String,
    pub id: u32,
    pub side: Side,
    pub required: bool,
    pub default: bool,
    pub deps: Vec<String>,
    pub filename: String,
    pub encoded: String,
    pub page: String,
    pub src: String,
    pub size: u64,
    pub md5: String,
    pub sha256: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CurseModFile {
    pub id: u32,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileDate")]
    pub file_date: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "gameVersion")]
    pub game_version: Vec<String>
}

impl std::fmt::Display for NixMod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,
r#"    "{slug}" = {{
        "title" = "{title}";
        "name" = "{slug}";
        "id" = {id};
        "side" = "{side}";
        "required" = {required};
        "default" = {default};
        "deps" = [];
        "filename" = "{filename}";
        "encoded" = "{encoded}";
        "page" = "{page}";
        "src" = "{src}";
        "type" = "remote";
        "md5" = "{md5}";
        "sha256" = "{sha256}";
        "size" = {size};
    }};"#,
            title = self.title,
            slug = self.slug,
            id = self.id,
            side = json!(self.side).as_str().unwrap(),
            required = self.required,
            default = self.default,
            filename = self.filename,
            encoded = self.encoded,
            page = self.page,
            src = self.src,
            md5 = self.md5,
            sha256 = self.sha256,
            size = self.size)
    }
}

impl YamlModFile {
    pub fn with_id(id: u32) -> YamlModFile {
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
    pub fn with_files(name: &str, id: u32, file: YamlModFile) -> YamlMod {
        YamlMod {
            name: name.to_owned(),
            id: id,
            side: None,
            required: None,
            default: None,
            files: Some(vec![file])
        }
    }
}

impl YamlManifest {
    pub fn merge(&self, others: Vec<YamlManifest>) -> YamlManifest {
        let mut mod_list = HashMap::new();
        let mut imports: HashSet<&String> = HashSet::new();
        for a_mod in &self.mods {
            mod_list.entry(&a_mod.name).or_insert(a_mod);
        }
        for other in &others {
            imports.extend(other.imports.iter());
            for a_mod in &other.mods {
                mod_list.entry(&a_mod.name).or_insert(a_mod);
            }
        }

        YamlManifest {
            version: self.version.clone(),
            imports: imports.into_iter().cloned().collect(),
            mods: mod_list.values().map(|&s| s.clone()).collect(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_merge_manifests() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let a_manifest_path = dir.path().join("a.yaml");
        let b_manifest_path = dir.path().join("b.yaml");
        let c_manifest_path = dir.path().join("c.yaml");

        write_yaml_manifest(&File::create(&a_manifest_path)?, vec!["b.yaml".to_string(), "c.yaml".to_string()], vec![
            YamlMod::with_id("jei", 238222),
            YamlMod::with_name("iron-chests")
        ])?;
        write_yaml_manifest(&File::create(&b_manifest_path)?, vec!["c.yaml".to_string()], vec![
            YamlMod::with_id("iron-chests", 123456)
        ])?;
        write_yaml_manifest(&File::create(&c_manifest_path)?, vec![], vec![
            YamlMod::with_files("waystones", 245755, YamlModFile::with_id(2859589))
        ])?;

        let merged_manifest = YamlManifest::recursive_load_from_file(&a_manifest_path)?;

        assert_eq!(merged_manifest.version, "1.12.2", "Should have correct version");
        assert_eq!(merged_manifest.imports.len(), 0, "Should have no remaining imports");
        assert_eq!(merged_manifest.mods.len(), 3, "Should exclude duplicates");
        assert!(merged_manifest.mods.iter().find(|&ref x| x.name == "iron-chests").unwrap().id.is_none(), "Higher level manifests should take priority");

        Ok(())
    }

    impl YamlMod {
        fn with_id(name: &str, id: u32) -> YamlMod {
            YamlMod {
                name: name.to_owned(),
                id: Some(id),
                side: None,
                required: None,
                default: None,
                files: None
            }
        }

        fn with_name(name: &str) -> YamlMod {
            YamlMod {
                name: name.to_owned(),
                id: None,
                side: None,
                required: None,
                default: None,
                files: None
            }
        }
    }

    fn write_yaml_manifest(file: &File, imports: Vec<String>, mods: Vec<YamlMod>) -> Result<()> {
        serde_yaml::to_writer(file, &YamlManifest {
            version: "1.12.2".to_string(),
            imports,
            mods
        })?;

        Ok(())
    }
}
