use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::DbPool;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct CustomRecipe {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: String,
    pub variables: String, // JSON array
    pub steps: String,     // JSON array
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeInput {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: String,
    pub variables: String,
    pub steps: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecipeInput {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: String,
    pub variables: String,
    pub steps: String,
}

pub async fn list_recipes(pool: &DbPool) -> Result<Vec<CustomRecipe>> {
    let rows = sqlx::query_as::<_, CustomRecipe>(
        "SELECT id, name, description, icon, tags, variables, steps, sync_status, created_at, updated_at FROM custom_recipes WHERE sync_status != 'deleted' ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_recipe(pool: &DbPool, id: &str) -> Result<CustomRecipe> {
    let recipe = sqlx::query_as::<_, CustomRecipe>(
        "SELECT id, name, description, icon, tags, variables, steps, sync_status, created_at, updated_at FROM custom_recipes WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .context(format!("Recipe not found: {}", id))?;
    Ok(recipe)
}

pub async fn create_recipe(pool: &DbPool, input: CreateRecipeInput) -> Result<CustomRecipe> {
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO custom_recipes (id, name, description, icon, tags, variables, steps, sync_status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.icon)
    .bind(&input.tags)
    .bind(&input.variables)
    .bind(&input.steps)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_recipe(pool, &id).await
}

pub async fn update_recipe(pool: &DbPool, id: &str, input: UpdateRecipeInput) -> Result<CustomRecipe> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE custom_recipes SET name=?, description=?, icon=?, tags=?, variables=?, steps=?, sync_status='pending', updated_at=? WHERE id=?",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.icon)
    .bind(&input.tags)
    .bind(&input.variables)
    .bind(&input.steps)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_recipe(pool, id).await
}

pub async fn delete_recipe(pool: &DbPool, id: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE custom_recipes SET sync_status = 'deleted', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
