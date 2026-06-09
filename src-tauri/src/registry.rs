use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::db::{registry as cache, DbPool};

const BASE_API_URL: &str =
    "https://api.github.com/repos/yolkmonday/shello-registry/contents";

fn contents_url(filename: &str) -> String {
    format!("{}/{}", BASE_API_URL, filename)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub files: HashMap<String, FileEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileEntry {
    pub version: String,
    pub filename: String,
}

/// GitHub Contents API response
#[derive(Debug, Deserialize)]
struct GitHubContent {
    content: String,
}

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("shello-app")
        .build()
        .context("Failed to build HTTP client")
}

/// Fetch a file from GitHub Contents API, returning decoded content.
async fn fetch_github_file(client: &reqwest::Client, filename: &str) -> Result<String> {
    let url = contents_url(filename);
    let resp: GitHubContent = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .json()
        .await?;

    // GitHub returns base64 content with newlines
    let cleaned = resp.content.replace('\n', "");
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &cleaned,
    )?;
    Ok(String::from_utf8(bytes)?)
}

/// Sync a single registry key. Returns the (possibly updated) JSON string.
async fn sync_key(
    client: &reqwest::Client,
    pool: &DbPool,
    key: &str,
    entry: &FileEntry,
) -> Result<String> {
    // Check local cache
    if let Some(cached) = cache::get_cache(pool, key).await? {
        if cached.version == entry.version {
            return Ok(cached.data);
        }
    }

    // Fetch new data
    let data = fetch_github_file(client, &entry.filename).await?;

    // Validate JSON
    serde_json::from_str::<serde_json::Value>(&data)?;

    cache::set_cache(pool, key, &entry.version, &data).await?;
    Ok(data)
}

/// Fetch manifest and sync all keys. Returns map of key -> JSON data.
pub async fn sync_all(pool: &DbPool) -> Result<HashMap<String, String>> {
    let client = build_client()?;
    let manifest_json = fetch_github_file(&client, "manifest.json").await?;
    let manifest: Manifest = serde_json::from_str(&manifest_json)?;

    let mut results = HashMap::new();
    for (key, entry) in &manifest.files {
        match sync_key(&client, pool, key, entry).await {
            Ok(data) => {
                results.insert(key.clone(), data);
            }
            Err(e) => {
                log::warn!("Failed to sync registry key '{}': {}", key, e);
                // Fall back to cache
                if let Ok(Some(cached)) = cache::get_cache(pool, key).await {
                    results.insert(key.clone(), cached.data);
                }
            }
        }
    }
    Ok(results)
}

/// Get cached data for a key without fetching from remote.
pub async fn get_cached(pool: &DbPool, key: &str) -> Result<Option<String>> {
    Ok(cache::get_cache(pool, key).await?.map(|e| e.data))
}
