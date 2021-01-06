use std::fs::create_dir_all;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::sync::Mutex;

const DB_NAME: &str = "cache.db";

#[derive(Debug)]
pub struct Query {
    url: String,
    // Primary key
    result: String,
    downloaded: SystemTime,  // This is stored as seconds since epoch
}

pub struct Database {
    lock: Mutex<Connection>,
}

fn setup(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS curse_queries (
                       url TEXT PRIMARY KEY,
                       result STRING NOT NULL,
                       downloaded INTEGER NOT NULL
                       )", params![])?;
    Ok(())
}

impl Database {
    pub fn from_filesystem() -> Result<Self> {
        let mut db_path = directories::ProjectDirs::from("brage.info", "erisia", "cursetool-rs")
            .context("While acquiring cache directory")?
            .cache_dir()
            .to_path_buf();
        log::info!("Using database path {:?}", db_path);
        create_dir_all(&db_path)
            .context(format!("While creating {:?}", &db_path))?;
        db_path.push(DB_NAME);
        let conn = Connection::open(&db_path)
            .context(format!("While opening {:?}", &db_path))?;
        setup(&conn)?;
        Ok(Database { lock: Mutex::new(conn) })
    }

    #[cfg(test)]
    pub fn for_tests() -> Result<Self> {
        log::info!("Using in-memory database");
        let conn = Connection::open_in_memory()?;
        setup(&conn)?;
        Ok(Database { lock: Mutex::new(conn) })
    }

    pub fn get_or_put<F>(&self, url: &str, lifetime: &Duration, downloader: F) -> Result<String>
        where F: FnOnce() -> Result<String> {
        let cached_result = {
            let conn = self.lock.lock().unwrap();
            let mut extract = conn.prepare_cached("SELECT result FROM curse_queries WHERE url = ? AND downloaded > ?")?;
            // We accept previously fetched data that's no older than valid_from.
            let valid_from = SystemTime::now() - *lifetime;
            // And convert that to seconds-since-epoch for use in SELECT.
            let limit_secs = valid_from.duration_since(UNIX_EPOCH)?.as_secs();

            let mut result = extract.query(params![url, limit_secs as i64])
                .context("Searching cache")?;

            result.next()?.map(|row| row.get(0))
        };

        if let Some(result) = cached_result {
            // Cache hit.
            Ok(result?)
        } else {
            // Cache miss. Recompute and insert.
            let conn = self.lock.lock().unwrap();
            let downloaded_at = SystemTime::now();
            let result = downloader()?;
            let mut update = conn.prepare_cached("INSERT OR REPLACE INTO curse_queries(url, result, downloaded) VALUES(?, ?, ?)")
                .context("Updating cache")?;
            update.execute(params![url, result, downloaded_at.duration_since(UNIX_EPOCH)?.as_secs() as i64])?;
            Ok(result)
        }
    }
}
