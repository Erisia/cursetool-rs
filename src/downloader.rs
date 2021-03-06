use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Url;
use reqwest::blocking::{Client, RequestBuilder};
use sha2::{Digest, Sha256};

use crate::database::Database;
use crate::model::{AddonInfo, CurseModFile, CurseModFileInfo};

static DEFAULT_TIMEOUT: Duration = Duration::from_secs(86400);
static INFINITE_TIMEOUT: Duration = Duration::from_secs(86400 * 365);
lazy_static! {
    static ref BASE_URL: Url = Url::parse("https://addons-ecs.forgesvc.net/api/v2/").unwrap();
}
// TODO: Implement with tokio.
//static MAX_CONCURRENT_QUERIES: u32 = 2;

pub struct Downloader<'app> {
    cache_timeout: Duration,
    client: Client,
    database: &'app Database,
    rate_limiter: Mutex<()>,
}

impl<'app> Downloader<'app> {
    pub(crate) fn request_mod_file_info(&self, download_url: &str) -> Result<CurseModFileInfo> {
        let mut download_url = Url::parse(download_url)?;
        // Edge URL don't work, for whatever reason.
        if let Some(host) = download_url.host_str() {
            if host == "edge.forgecdn.net" {
                download_url.set_host(Some("media.forgecdn.net"))?;
            }
        } else {
            anyhow::bail!("download_url missing host part!");
        }
        // We can generally assume files don't change.
        let json = self.database.get_or_put(&download_url.as_str(), &INFINITE_TIMEOUT, || {
            let mut buf: Vec<u8> = vec![];
            let mut body = reqwest::blocking::get(download_url.clone())?;
            let content_type = body.headers().get("content-type")
                .context("Reading content-type")?;
            if content_type == "application/xml" {
                anyhow::bail!("Miscomputed URL! {} returned XML", download_url.as_str());
            }
            let size = body.copy_to(&mut buf)?;
            let md5 = format!("{:x}", md5::compute(&buf));
            let sha256 = format!("{:x}", Sha256::digest(&buf));
            let mod_info = CurseModFileInfo { md5, sha256, size, download_url: download_url.to_string() };
            Ok(serde_json::to_string(&mod_info)?)
        })?;
        Ok(serde_json::from_str(&json)?)
    }
}

impl<'app> Downloader<'app> {
    pub(crate) fn request_mod_files(&self, project_id: u32) -> Result<Vec<CurseModFile>> {
        let url = BASE_URL
            .join(&format!("addon/{}/files", project_id))?;
        let data = self.get(url.clone())
            .context(format!("Fetching files for project id {}", project_id))?;
        let result: Vec<CurseModFile> = serde_json::from_str(&data)
            .context(format!("Parsing files list as JSON for project id {}", project_id))?;
        // The URLs returned are not properly URL-encoded.
        // Specifically, the filename path needs to be encoded.
        //
        // Breaking the URL spec, curseforge requires + to be encoded.
        // This means we need to do the job 'manually'.
        result
            .into_iter()
            .map(|file| {
                let url = Url::parse(&file.download_url).unwrap();
                let filename = url.path_segments()
                    .unwrap()
                    .last()
                    .unwrap();
                // Sometimes the download URL is already encoded, and sometimes not.
                // This encoder gives working output for the filename.
                let encoded_filename = urlencoding::encode(
                    &urlencoding::decode(filename).unwrap());
                // We now need to construct a new url, *not* re-encoding it.
                let mut base_url = url.clone();
                base_url.path_segments_mut().unwrap()
                    .pop();
                let fixed_url = Url::parse(
                    &format!("{}/{}", base_url.as_str(), &encoded_filename)).unwrap();
                Ok(CurseModFile {
                    download_url: fixed_url.to_string(),
                    ..file
                })
            })
        .collect()
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

    fn get_with_builder<F>(&self, url: Url, f: F) -> Result<String> where F: FnOnce(RequestBuilder) -> RequestBuilder {
        let request = f(self.client.get(url)).build()?;
        let url: String = request.url().as_str().into();
        self.database.get_or_put(&url, &self.cache_timeout, || {
            let _guard = self.rate_limiter.lock().unwrap();
            log::debug!("Fetching {}", url);
            Ok(self.client.execute(request)?.text()?)
        })
    }

    fn get(&self, url: Url) -> Result<String> {
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

    pub(crate) fn request_addon_info(&self, project_id: u32) -> Result<AddonInfo> {
        let url = BASE_URL
            .join(&format!("addon/{}", project_id))?;
        let data = self.get_with_builder(url.clone(), |b| b)
                .context(format!("Fetching addon info for project id {}", project_id))
                .context(format!("From {:?}", url.as_str()))?;
        serde_json::from_str(&data)
                .context(format!("Parsing addon info as JSON for project id {}. Data: {}", project_id, data))
                .context(format!("From {}", url.as_str()))
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
