use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub version: String
}
#[derive(Serialize, Deserialize)]
pub struct ModFile {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    pub required: bool
}
#[derive(Serialize, Deserialize)]
pub struct CurseManifest {
    pub minecraft: MinecraftVersion,
    pub files: Vec<ModFile>
}
#[derive(Serialize, Deserialize)]
pub struct AddonInfo {
    pub name: String,
    #[serde(rename = "websiteUrl")]
    pub website_url: String
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct YamlMod {
    pub name: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub files: Option<Vec<YamlModFile>>
}

#[derive(Serialize, Deserialize)]
pub struct YamlManifest {
    pub version: String,
    pub imports: Vec<String>,
    pub mods: Vec<YamlMod>
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
    pub fn with_id_name_src(id: u32, name: &str, src: &str) -> YamlModFile {
        YamlModFile {
            name: Some(name.to_owned()),
            id: Some(id),
            maturity: None,
            file_page_url: None,
            src: Some(src.to_owned()),
            md5: None
        }
    }
}

impl YamlMod {
    pub fn with_files(name: &str, file: YamlModFile) -> YamlMod {
        YamlMod {
            name: name.to_owned(),
            side: None,
            required: None,
            default: None,
            files: Some(vec![file])
        }
    }
}
