use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::DbPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub tags: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSnippetInput {
    pub name: String,
    pub command: String,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSnippetInput {
    pub name: Option<String>,
    pub command: Option<String>,
    pub tags: Option<String>,
}

pub async fn list_snippets(pool: &DbPool) -> Result<Vec<Snippet>> {
    let snippets = sqlx::query_as::<_, Snippet>(
        "SELECT * FROM snippets WHERE sync_status != 'deleted' ORDER BY sort_order, name"
    )
    .fetch_all(pool)
    .await?;
    Ok(snippets)
}

pub async fn create_snippet(pool: &DbPool, input: CreateSnippetInput) -> Result<Snippet> {
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let tags = input.tags.unwrap_or_default();

    sqlx::query(
        "INSERT INTO snippets (id, name, command, tags, sort_order, sync_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, 'pending', ?, ?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.command)
    .bind(&tags)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_snippet(pool, &id).await
}

pub async fn get_snippet(pool: &DbPool, id: &str) -> Result<Snippet> {
    let snippet = sqlx::query_as::<_, Snippet>("SELECT * FROM snippets WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context(format!("Snippet not found: {}", id))?;
    Ok(snippet)
}

pub async fn update_snippet(pool: &DbPool, id: &str, input: UpdateSnippetInput) -> Result<Snippet> {
    let existing = get_snippet(pool, id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE snippets SET name = ?, command = ?, tags = ?, sync_status = 'pending', updated_at = ? WHERE id = ?"
    )
    .bind(input.name.unwrap_or(existing.name))
    .bind(input.command.unwrap_or(existing.command))
    .bind(input.tags.unwrap_or(existing.tags))
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_snippet(pool, id).await
}

pub async fn delete_snippet(pool: &DbPool, id: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE snippets SET sync_status = 'deleted', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
