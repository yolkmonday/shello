use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tunnel {
    pub id: String,
    pub profile_id: String,
    pub tunnel_type: String,
    pub local_host: String,
    pub local_port: i64,
    pub remote_host: String,
    pub remote_port: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTunnelInput {
    pub profile_id: String,
    pub local_host: Option<String>,
    pub local_port: i64,
    pub remote_host: String,
    pub remote_port: i64,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTunnelInput {
    pub local_host: Option<String>,
    pub local_port: Option<i64>,
    pub remote_host: Option<String>,
    pub remote_port: Option<i64>,
    pub enabled: Option<bool>,
}

/// Validate a tunnel's ports and remote host. Used by create/update.
pub fn validate_tunnel(local_port: i64, remote_port: i64, remote_host: &str) -> Result<()> {
    if !(1..=65535).contains(&local_port) {
        bail!("Local port must be between 1 and 65535");
    }
    if !(1..=65535).contains(&remote_port) {
        bail!("Remote port must be between 1 and 65535");
    }
    if remote_host.trim().is_empty() {
        bail!("Remote host is required");
    }
    Ok(())
}

pub async fn list_tunnels(pool: &DbPool, profile_id: &str) -> Result<Vec<Tunnel>> {
    let tunnels = sqlx::query_as::<_, Tunnel>(
        "SELECT * FROM tunnels WHERE profile_id = ? ORDER BY local_port",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(tunnels)
}

pub async fn list_enabled_tunnels(pool: &DbPool, profile_id: &str) -> Result<Vec<Tunnel>> {
    let tunnels = sqlx::query_as::<_, Tunnel>(
        "SELECT * FROM tunnels WHERE profile_id = ? AND enabled = 1 ORDER BY local_port",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(tunnels)
}

pub async fn get_tunnel(pool: &DbPool, id: &str) -> Result<Tunnel> {
    let tunnel = sqlx::query_as::<_, Tunnel>("SELECT * FROM tunnels WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context(format!("Tunnel not found: {}", id))?;
    Ok(tunnel)
}

pub async fn create_tunnel(pool: &DbPool, input: CreateTunnelInput) -> Result<Tunnel> {
    validate_tunnel(input.local_port, input.remote_port, &input.remote_host)?;
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let local_host = input.local_host.unwrap_or_else(|| "127.0.0.1".to_string());
    let enabled = input.enabled.unwrap_or(true);

    sqlx::query(
        "INSERT INTO tunnels (id, profile_id, tunnel_type, local_host, local_port, remote_host, remote_port, enabled, created_at, updated_at)
         VALUES (?, ?, 'local', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.profile_id)
    .bind(&local_host)
    .bind(input.local_port)
    .bind(&input.remote_host)
    .bind(input.remote_port)
    .bind(enabled)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_tunnel(pool, &id).await
}

pub async fn update_tunnel(pool: &DbPool, id: &str, input: UpdateTunnelInput) -> Result<Tunnel> {
    let existing = get_tunnel(pool, id).await?;
    let local_host = input.local_host.unwrap_or(existing.local_host);
    let local_port = input.local_port.unwrap_or(existing.local_port);
    let remote_host = input.remote_host.unwrap_or(existing.remote_host);
    let remote_port = input.remote_port.unwrap_or(existing.remote_port);
    let enabled = input.enabled.unwrap_or(existing.enabled);
    validate_tunnel(local_port, remote_port, &remote_host)?;
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE tunnels SET local_host = ?, local_port = ?, remote_host = ?, remote_port = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&local_host)
    .bind(local_port)
    .bind(&remote_host)
    .bind(remote_port)
    .bind(enabled)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_tunnel(pool, id).await
}

pub async fn delete_tunnel(pool: &DbPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM tunnels WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tunnel_ok() {
        assert!(validate_tunnel(5432, 5432, "db.internal").is_ok());
    }

    #[test]
    fn rejects_bad_local_port() {
        assert!(validate_tunnel(0, 80, "h").is_err());
        assert!(validate_tunnel(70000, 80, "h").is_err());
    }

    #[test]
    fn rejects_bad_remote_port() {
        assert!(validate_tunnel(80, 0, "h").is_err());
    }

    #[test]
    fn rejects_empty_remote_host() {
        assert!(validate_tunnel(80, 80, "   ").is_err());
    }
}
