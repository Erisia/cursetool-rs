use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::blocking::{Client, RequestBuilder};
use sha2::{Digest, Sha256};

use crate::database::Database;
use crate::model::{AddonInfo, CurseModFile, CurseModFileInfo};

static DEFAULT_TIMEOUT: Duration = Duration::from_secs(86400);
static INFINITE_TIMEOUT: Duration = Duration::from_secs(86400 * 365);
static BASE_URL: &str = "https://addons-ecs.forgesvc.net/api/v2";
// TODO: Implement with tokio.
//static MAX_CONCURRENT_QUERIES: u32 = 2;

pub struct Downloader<'app> {
    cache_timeout: Duration,
    client: Client,
    database: &'app Database,
    rate_limiter: Mutex<()>,
}

impl<'app> Downloader<'app> {
    pub(crate) fn request_mod_file_info(&self, download_url: &String) -> Result<CurseModFileInfo> {
        let redirected_url = download_url.replace("edge.forgecdn.net", "media.forgecdn.net");
        // We can generally assume files don't change.
        let json = self.database.get_or_put(&redirected_url, &INFINITE_TIMEOUT, || {
            let mut buf: Vec<u8> = vec![];
            let size = reqwest::blocking::get(&redirected_url)?.copy_to(&mut buf)?;
            let md5 = format!("{:x}", md5::compute(&buf));
            let sha256 = format!("{:x}", Sha256::digest(&buf));
            let mod_info = CurseModFileInfo { md5, sha256, size, download_url: download_url.clone()  };
            Ok(serde_json::to_string(&mod_info)?)
        })?;
        Ok(serde_json::from_str(&json)?)
    }
}

impl<'app> Downloader<'app> {
    pub(crate) fn request_mod_files(&self, project_id: u32) -> Result<Vec<CurseModFile>> {
        let url = format!("{}/addon/{}/files", BASE_URL, project_id);
        let data = self.get(&url)
            .context(format!("Fetching files for project id {}", project_id))?;
        serde_json::from_str(&data)
            .context("Parsing files list as JSON")
    }
}

impl<'app> Downloader<'app> {
    pub fn new(database: &'app Database) -> Self {
        Downloader {
            cache_timeout: DEFAULT_TIMEOUT,
            client: Client::new(),
            database,
            rate_limiter: Mutex::new(()),
        }
    }

    fn get_with_builder<F>(&self, url: &String, f: F) -> Result<String> where F: FnOnce(RequestBuilder) -> RequestBuilder {
        let request = f(self.client.get(url)).build()?;
        let url: String = request.url().as_str().into();
        self.database.get_or_put(&url, &self.cache_timeout, || {
            let _guard = self.rate_limiter.lock().unwrap();
            log::debug!("Fetching {}", url);
            Ok(self.client.execute(request)?.text()?)
        })
    }

    fn get(&self, url: &String) -> Result<String> {
        self.get_with_builder(url, |b| b)
    }

    pub(crate) fn get_slug_from_webpage_url(&self, url: &str) -> Result<String> {
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

    pub(crate) fn request_mod_listing(&self, version: &str) -> Result<HashMap<String, u32>> {
        const MAX_MOD_COUNT: usize = 10_000;
        let url = format!("{}/addon/search", BASE_URL);
        let response: String = self.get_with_builder(
            &url,
            |builder| builder.query(&[
                ("gameId", "432"),
                ("gameVersion", version),
                ("sort", "3"),
                ("sectionId", "6"),
                ("pageSize", &MAX_MOD_COUNT.to_string())]) // Less than 9000 1.12.2 mods as of 2021-01-01
        )?;
        let response: Vec<AddonInfo> = serde_json::from_str(&response)?;

        if response.len() >= MAX_MOD_COUNT {
            bail!("The first page of results is full, some mods may not be present in list.");
        }
        let mut result = HashMap::new();
        for addon in response {
            let slug = self.get_slug_from_webpage_url(&addon.website_url)
                .context(format!("Fetching slug for {:?}", addon))?;
            result.insert(slug, addon.id);
        }
        Ok(result)
    }

    pub(crate) fn request_addon_info(&self, project_id: u32) -> Result<AddonInfo> {
        let url = format!("{}/addon/{}", BASE_URL, project_id);
        let data = self.get_with_builder(&url, |b| b)
                .context(format!("Fetching addon info for project id {}", project_id))?;
        serde_json::from_str(&data)
                .context("Parsing addon info as JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_downloader<F, X>(f: F) -> Result<X>
        where F: FnOnce(Downloader) -> Result<X> {
        let database = Database::for_tests().unwrap();
        f(Downloader::new(&database))
    }

    #[test]
    fn can_get_slug_from_url() {
        let url = "https://www.curseforge.com/minecraft/mc-mods/hunger-overhaul";
        let result = with_downloader(|d| d.get_slug_from_webpage_url(url)).unwrap();

        assert_eq!(result, "hunger-overhaul");
    }

    #[test]
    fn can_get_addon_info() {
        let project_id = 224476; // Hunger Overhaul
        let result: AddonInfo = with_downloader(|d| d.request_addon_info(project_id)).unwrap();

        assert_eq!(result.name, "Hunger Overhaul");
        assert_eq!(result.id, project_id);
        assert!(result.website_url.contains("hunger-overhaul"));
    }

}