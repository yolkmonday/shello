use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::DbPool;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub key: String,
    pub version: String,
    pub data: String,
    pub fetched_at: String,
}

pub async fn get_cache(pool: &DbPool, key: &str) -> Result<Option<RegistryEntry>> {
    let entry = sqlx::query_as::<_, RegistryEntry>(
        "SELECT key, version, data, fetched_at FROM registry_cache WHERE key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(entry)
}

pub async fn set_cache(pool: &DbPool, key: &str, version: &str, data: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO registry_cache (key, version, data, fetched_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET version = excluded.version, data = excluded.data, fetched_at = excluded.fetched_at",
    )
    .bind(key)
    .bind(version)
    .bind(data)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
