use serde::{Serialize, Deserialize};
use serde_json::json;
use std::collections::HashSet;

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
    #[serde(skip_serializing_if="Option::is_none")]
    pub id: Option<u32>,
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
        "deps" = {deps};
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
            deps = "[]",
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
            id: Some(id),
            side: None,
            required: None,
            default: None,
            files: Some(vec![file])
        }
    }
}

impl YamlManifest {
    // probably way too much cloning here
    pub fn merge(&self, others: Vec<YamlManifest>) -> YamlManifest {
        let mut mod_list: Vec<YamlMod> = self.mods.to_owned();
        let mut imports: HashSet<String> = HashSet::new();
        for other_manifest in &others {
            imports.extend(other_manifest.imports.clone().into_iter());
            for m in &other_manifest.clone().mods {
                if ! mod_list.clone().into_iter().any(|i: YamlMod| i.name == m.name) {
                    mod_list.push(m.clone());
                }
            }
        }

        YamlManifest {
            version: self.version.clone(),
            imports: imports.into_iter().collect(),
            mods: mod_list
        }
    }
}
