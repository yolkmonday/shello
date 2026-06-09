pub mod custom_recipes;
pub mod profiles;
pub mod registry;
pub mod snippets;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub type DbPool = SqlitePool;

pub async fn init(app_data_dir: &std::path::Path) -> Result<DbPool> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("shello.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("src/db/migrations").run(&pool).await?;
    Ok(pool)
}
