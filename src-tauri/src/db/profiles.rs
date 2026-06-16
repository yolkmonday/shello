use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::DbPool;
use crate::crypto;
use crate::vault::VaultState;

// ── Types ────────────────────────────────────────────────────────────

#[derive(Debug, FromRow)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub password_enc: Option<Vec<u8>>,
    pub key_path_enc: Option<Vec<u8>>,
    pub passphrase_enc: Option<Vec<u8>>,
    pub group_id: Option<String>,
    pub tags: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Profile without credentials — safe to send to frontend.
#[derive(Debug, Serialize, FromRow)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub group_id: Option<String>,
    pub tags: String,
    pub sort_order: i64,
    pub detected_os: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub passphrase: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileInput {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub auth_type: Option<String>,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub passphrase: Option<String>,
    /// None = don't change, Some(None) = set to null, Some(Some(id)) = set to id
    pub group_id: Option<Option<String>>,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ── Credential encryption helpers ───────────────────────────────────

/// Encrypt a credential string if vault is initialized and unlocked.
/// If vault is not initialized, stores as plaintext bytes.
/// If vault is locked, returns error.
fn encrypt_credential(value: Option<&str>, vault_initialized: bool, key: Option<&[u8; 32]>) -> Result<Option<Vec<u8>>> {
    match value {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            if vault_initialized {
                let k = key.ok_or_else(|| anyhow::anyhow!("vault_locked"))?;
                Ok(Some(crypto::encrypt(s.as_bytes(), k)?))
            } else {
                Ok(Some(s.as_bytes().to_vec()))
            }
        }
    }
}

/// Decrypt a credential blob. If vault is not initialized, treats blob as plaintext bytes.
fn decrypt_credential(blob: Option<&[u8]>, vault_initialized: bool, key: Option<&[u8; 32]>) -> Result<Option<String>> {
    match blob {
        None => Ok(None),
        Some(b) if b.is_empty() => Ok(None),
        Some(b) => {
            if vault_initialized {
                let k = key.ok_or_else(|| anyhow::anyhow!("vault_locked"))?;
                let plain = crypto::decrypt(b, k)?;
                Ok(Some(String::from_utf8(plain)?))
            } else {
                Ok(Some(String::from_utf8_lossy(b).into_owned()))
            }
        }
    }
}

// ── Group CRUD ───────────────────────────────────────────────────────

pub async fn list_groups(pool: &DbPool) -> Result<Vec<Group>> {
    let groups = sqlx::query_as::<_, Group>(
        "SELECT id, name, color, sort_order, sync_status, remote_updated_at, created_at, updated_at
         FROM groups WHERE sync_status != 'deleted' ORDER BY sort_order, name"
    )
    .fetch_all(pool)
    .await?;
    Ok(groups)
}

pub async fn create_group(pool: &DbPool, name: &str, color: &str) -> Result<Group> {
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO groups (id, name, color, sort_order, sync_status, created_at, updated_at)
         VALUES (?, ?, ?, 0, 'pending', ?, ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(color)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_group(pool, &id).await
}

pub async fn get_group(pool: &DbPool, id: &str) -> Result<Group> {
    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context(format!("Group not found: {}", id))?;
    Ok(group)
}

pub async fn update_group(pool: &DbPool, id: &str, name: &str, color: &str) -> Result<Group> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE groups SET name = ?, color = ?, sync_status = 'pending', updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(color)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

    get_group(pool, id).await
}

pub async fn delete_group(pool: &DbPool, id: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE profiles SET group_id = NULL, sync_status = 'pending', updated_at = ? WHERE group_id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE groups SET sync_status = 'deleted', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Profile CRUD ─────────────────────────────────────────────────────

pub async fn list_profiles(pool: &DbPool) -> Result<Vec<ProfileSummary>> {
    let profiles = sqlx::query_as::<_, ProfileSummary>(
        "SELECT id, name, host, port, username, auth_type, group_id, tags, sort_order, detected_os, created_at, updated_at
         FROM profiles WHERE sync_status != 'deleted' ORDER BY sort_order, name"
    )
    .fetch_all(pool)
    .await?;
    Ok(profiles)
}

pub async fn get_profile(pool: &DbPool, id: &str) -> Result<Profile> {
    let profile = sqlx::query_as::<_, Profile>("SELECT * FROM profiles WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context(format!("Profile not found: {}", id))?;
    Ok(profile)
}

pub async fn create_profile(pool: &DbPool, input: CreateProfileInput, vault: &VaultState) -> Result<ProfileSummary> {
    let id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let port = input.port.unwrap_or(22);
    let tags = input.tags.unwrap_or_default();

    let vault_init = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vault_config")
        .fetch_one(pool).await? > 0;
    let key = if vault_init { Some(vault.get_key()?) } else { None };

    let pass_enc = encrypt_credential(input.password.as_deref(), vault_init, key.as_ref())?;
    let key_enc = encrypt_credential(input.key_path.as_deref(), vault_init, key.as_ref())?;
    let phrase_enc = encrypt_credential(input.passphrase.as_deref(), vault_init, key.as_ref())?;

    sqlx::query(
        "INSERT INTO profiles (id, name, host, port, username, auth_type, password_enc, key_path_enc, passphrase_enc, group_id, tags, sort_order, sync_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 'pending', ?, ?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.host)
    .bind(port)
    .bind(&input.username)
    .bind(&input.auth_type)
    .bind(&pass_enc)
    .bind(&key_enc)
    .bind(&phrase_enc)
    .bind(&input.group_id)
    .bind(&tags)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_profile_summary(pool, &id).await
}

/// Duplicate a profile, copying all fields (including encrypted credentials)
/// into a new row named "<name> copy".
pub async fn duplicate_profile(pool: &DbPool, id: &str) -> Result<ProfileSummary> {
    let src = get_profile(pool, id).await?;
    let new_id = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let new_name = format!("{} copy", src.name);

    sqlx::query(
        "INSERT INTO profiles (id, name, host, port, username, auth_type, password_enc, key_path_enc, passphrase_enc, group_id, tags, sort_order, sync_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 'pending', ?, ?)"
    )
    .bind(&new_id)
    .bind(&new_name)
    .bind(&src.host)
    .bind(src.port)
    .bind(&src.username)
    .bind(&src.auth_type)
    .bind(&src.password_enc)
    .bind(&src.key_path_enc)
    .bind(&src.passphrase_enc)
    .bind(&src.group_id)
    .bind(&src.tags)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_profile_summary(pool, &new_id).await
}

pub async fn get_profile_summary(pool: &DbPool, id: &str) -> Result<ProfileSummary> {
    let profile = sqlx::query_as::<_, ProfileSummary>(
        "SELECT id, name, host, port, username, auth_type, group_id, tags, sort_order, detected_os, created_at, updated_at
         FROM profiles WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .context(format!("Profile not found: {}", id))?;
    Ok(profile)
}

/// Get a profile and decrypt its credentials.
pub async fn get_profile_decrypted(pool: &DbPool, id: &str, vault: &VaultState) -> Result<(String, String, Option<String>)> {
    let profile = get_profile(pool, id).await?;
    let vault_init = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vault_config")
        .fetch_one(pool).await? > 0;
    let key = if vault_init { Some(vault.get_key()?) } else { None };

    let password = decrypt_credential(profile.password_enc.as_deref(), vault_init, key.as_ref())?
        .unwrap_or_default();
    let key_path = decrypt_credential(profile.key_path_enc.as_deref(), vault_init, key.as_ref())?
        .unwrap_or_default();
    let passphrase = decrypt_credential(profile.passphrase_enc.as_deref(), vault_init, key.as_ref())?;

    Ok((password, key_path, passphrase))
}

pub async fn update_profile(pool: &DbPool, id: &str, input: UpdateProfileInput, vault: &VaultState) -> Result<ProfileSummary> {
    let existing = get_profile(pool, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let auth_type = input.auth_type.clone().unwrap_or(existing.auth_type.clone());

    let vault_init = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vault_config")
        .fetch_one(pool).await? > 0;
    let key = if vault_init && (input.password.is_some() || input.key_path.is_some() || input.passphrase.is_some()) {
        Some(vault.get_key()?)
    } else { None };

    sqlx::query(
        "UPDATE profiles SET name = ?, host = ?, port = ?, username = ?, auth_type = ?,
         password_enc = ?, key_path_enc = ?, passphrase_enc = ?, group_id = ?, tags = ?, sync_status = 'pending', updated_at = ?
         WHERE id = ?"
    )
    .bind(input.name.unwrap_or(existing.name))
    .bind(input.host.unwrap_or(existing.host))
    .bind(input.port.unwrap_or(existing.port))
    .bind(input.username.unwrap_or(existing.username))
    .bind(&auth_type)
    .bind(if auth_type == "password" {
        match input.password {
            Some(s) => encrypt_credential(Some(&s), vault_init, key.as_ref())?,
            None => existing.password_enc,
        }
    } else { None })
    .bind(if auth_type == "key" {
        match input.key_path {
            Some(s) => encrypt_credential(Some(&s), vault_init, key.as_ref())?,
            None => existing.key_path_enc,
        }
    } else { None })
    .bind(if auth_type == "key" {
        match input.passphrase {
            Some(s) => encrypt_credential(Some(&s), vault_init, key.as_ref())?,
            None => existing.passphrase_enc,
        }
    } else { None })
    .bind(input.group_id.unwrap_or(existing.group_id))
    .bind(input.tags.unwrap_or(existing.tags))
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_profile_summary(pool, id).await
}

pub async fn delete_profile(pool: &DbPool, id: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE profiles SET sync_status = 'deleted', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_detected_os(pool: &DbPool, id: &str, distro: &str) -> Result<()> {
    sqlx::query("UPDATE profiles SET detected_os = ? WHERE id = ?")
        .bind(distro)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn search_profiles(pool: &DbPool, query: &str) -> Result<Vec<ProfileSummary>> {
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{}%", escaped);
    let profiles = sqlx::query_as::<_, ProfileSummary>(
        "SELECT id, name, host, port, username, auth_type, group_id, tags, sort_order, detected_os, created_at, updated_at
         FROM profiles
         WHERE sync_status != 'deleted' AND (name LIKE ? ESCAPE '\\' OR host LIKE ? ESCAPE '\\' OR username LIKE ? ESCAPE '\\' OR tags LIKE ? ESCAPE '\\')
         ORDER BY sort_order, name"
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    Ok(profiles)
}
