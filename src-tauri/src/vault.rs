use std::sync::RwLock;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Serialize;
use sqlx::FromRow;

use crate::crypto;
use crate::db::DbPool;

const VERIFY_PLAINTEXT: &[u8] = b"shello-vault-ok";
const KEYCHAIN_SERVICE: &str = "shello";
const KEYCHAIN_ACCOUNT: &str = "vault-key";

// ── VaultState ──────────────────────────────────────────────────────

pub struct VaultState {
    derived_key: RwLock<Option<[u8; 32]>>,
}

impl VaultState {
    pub fn new() -> Self {
        Self {
            derived_key: RwLock::new(None),
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.derived_key.read().unwrap().is_some()
    }

    pub fn get_key(&self) -> Result<[u8; 32]> {
        self.derived_key
            .read()
            .unwrap()
            .ok_or_else(|| anyhow!("vault_locked"))
    }

    pub fn set_key(&self, key: [u8; 32]) {
        *self.derived_key.write().unwrap() = Some(key);
    }

    pub fn lock(&self) {
        let mut guard = self.derived_key.write().unwrap();
        if let Some(ref mut key) = *guard {
            key.fill(0);
        }
        *guard = None;
    }
}

// ── DB types ────────────────────────────────────────────────────────

#[derive(Debug, FromRow)]
struct VaultConfig {
    #[allow(dead_code)]
    id: i64,
    salt: Vec<u8>,
    verify_blob: Vec<u8>,
    #[allow(dead_code)]
    created_at: String,
}

#[derive(Debug, Serialize)]
pub struct VaultStatus {
    pub initialized: bool,
    pub unlocked: bool,
}

// ── DB helpers ──────────────────────────────────────────────────────

async fn get_vault_config(pool: &DbPool) -> Result<Option<VaultConfig>> {
    let config = sqlx::query_as::<_, VaultConfig>("SELECT * FROM vault_config WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(config)
}

async fn vault_is_initialized(pool: &DbPool) -> bool {
    get_vault_config(pool).await.ok().flatten().is_some()
}

// ── Keychain helpers ────────────────────────────────────────────────

fn keychain_store(key: &[u8; 32]) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| anyhow!("Keychain init failed: {}", e))?;
    let encoded = B64.encode(key);
    entry
        .set_password(&encoded)
        .map_err(|e| anyhow!("Keychain store failed: {}", e))?;
    Ok(())
}

fn keychain_load() -> Result<Option<[u8; 32]>> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| anyhow!("Keychain init failed: {}", e))?;
    match entry.get_password() {
        Ok(encoded) => {
            let bytes = B64.decode(&encoded)
                .map_err(|e| anyhow!("Keychain decode failed: {}", e))?;
            if bytes.len() != 32 {
                return Err(anyhow!("Invalid key length in keychain"));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow!("Keychain load failed: {}", e)),
    }
}

fn keychain_delete() -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| anyhow!("Keychain init failed: {}", e))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow!("Keychain delete failed: {}", e)),
    }
}

// ── Public functions (called from lib.rs setup) ─────────────────────

/// Try to auto-unlock vault from OS keychain. Never panics.
pub async fn try_auto_unlock(pool: &DbPool, vault: &VaultState) {
    if !vault_is_initialized(pool).await {
        return;
    }

    let key = match keychain_load().ok().flatten() {
        Some(k) => k,
        None => return,
    };

    if let Ok(Some(config)) = get_vault_config(pool).await {
        match crypto::decrypt(&config.verify_blob, &key) {
            Ok(plaintext) if plaintext == VERIFY_PLAINTEXT => {
                vault.set_key(key);
                log::info!("Vault auto-unlocked from keychain");
            }
            _ => {
                let _ = keychain_delete();
                log::warn!("Stale vault key in keychain, removed");
            }
        }
    }
}

// ── Tauri commands ──────────────────────────────────────────────────

#[tauri::command]
pub async fn vault_status(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
) -> Result<VaultStatus, String> {
    let initialized = vault_is_initialized(&pool).await;
    Ok(VaultStatus {
        initialized,
        unlocked: vault.is_unlocked(),
    })
}

#[tauri::command]
pub async fn vault_setup(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    master_password: String,
    remember: bool,
) -> Result<(), String> {
    if vault_is_initialized(&pool).await {
        return Err("Vault already initialized".into());
    }

    let salt = crypto::generate_salt();
    let key = crypto::derive_key(&master_password, &salt);
    let verify_blob = crypto::encrypt(VERIFY_PLAINTEXT, &key)
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("INSERT INTO vault_config (id, salt, verify_blob, created_at) VALUES (1, ?, ?, ?)")
        .bind(&salt[..])
        .bind(&verify_blob)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT id, password_enc, key_path_enc, passphrase_enc FROM profiles")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    for (id, pass, kpath, phrase) in rows {
        let enc_pass = pass
            .filter(|b| !b.is_empty())
            .map(|b| crypto::encrypt(&b, &key))
            .transpose()
            .map_err(|e| e.to_string())?;
        let enc_kpath = kpath
            .filter(|b| !b.is_empty())
            .map(|b| crypto::encrypt(&b, &key))
            .transpose()
            .map_err(|e| e.to_string())?;
        let enc_phrase = phrase
            .filter(|b| !b.is_empty())
            .map(|b| crypto::encrypt(&b, &key))
            .transpose()
            .map_err(|e| e.to_string())?;

        sqlx::query(
            "UPDATE profiles SET password_enc = ?, key_path_enc = ?, passphrase_enc = ? WHERE id = ?"
        )
        .bind(&enc_pass)
        .bind(&enc_kpath)
        .bind(&enc_phrase)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    vault.set_key(key);

    if remember {
        if let Err(e) = keychain_store(&key) {
            log::warn!("Failed to store key in keychain: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn vault_unlock(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    master_password: String,
    remember: bool,
) -> Result<(), String> {
    let config = get_vault_config(&pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Vault not initialized")?;

    let key = crypto::derive_key(&master_password, &config.salt);

    let plaintext = crypto::decrypt(&config.verify_blob, &key)
        .map_err(|_| "Wrong master password".to_string())?;

    if plaintext != VERIFY_PLAINTEXT {
        return Err("Wrong master password".into());
    }

    vault.set_key(key);

    if remember {
        if let Err(e) = keychain_store(&key) {
            log::warn!("Failed to store key in keychain: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn vault_lock(
    vault: tauri::State<'_, VaultState>,
) -> Result<(), String> {
    vault.lock();
    Ok(())
}

#[tauri::command]
pub async fn vault_change_password(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    let config = get_vault_config(&pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Vault not initialized")?;

    let old_key = crypto::derive_key(&current_password, &config.salt);
    let plaintext = crypto::decrypt(&config.verify_blob, &old_key)
        .map_err(|_| "Wrong current password".to_string())?;
    if plaintext != VERIFY_PLAINTEXT {
        return Err("Wrong current password".into());
    }

    let new_salt = crypto::generate_salt();
    let new_key = crypto::derive_key(&new_password, &new_salt);
    let new_verify = crypto::encrypt(VERIFY_PLAINTEXT, &new_key)
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("UPDATE vault_config SET salt = ?, verify_blob = ?, created_at = ? WHERE id = 1")
        .bind(&new_salt[..])
        .bind(&new_verify)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT id, password_enc, key_path_enc, passphrase_enc FROM profiles")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    for (id, pass, kpath, phrase) in rows {
        let re_pass = pass
            .filter(|b| !b.is_empty())
            .map(|b| {
                let plain = crypto::decrypt(&b, &old_key)?;
                crypto::encrypt(&plain, &new_key)
            })
            .transpose()
            .map_err(|e: anyhow::Error| e.to_string())?;
        let re_kpath = kpath
            .filter(|b| !b.is_empty())
            .map(|b| {
                let plain = crypto::decrypt(&b, &old_key)?;
                crypto::encrypt(&plain, &new_key)
            })
            .transpose()
            .map_err(|e: anyhow::Error| e.to_string())?;
        let re_phrase = phrase
            .filter(|b| !b.is_empty())
            .map(|b| {
                let plain = crypto::decrypt(&b, &old_key)?;
                crypto::encrypt(&plain, &new_key)
            })
            .transpose()
            .map_err(|e: anyhow::Error| e.to_string())?;

        sqlx::query(
            "UPDATE profiles SET password_enc = ?, key_path_enc = ?, passphrase_enc = ? WHERE id = ?"
        )
        .bind(&re_pass)
        .bind(&re_kpath)
        .bind(&re_phrase)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    vault.set_key(new_key);

    // Only update keychain if it already had a key stored
    if keychain_load().ok().flatten().is_some() {
        let _ = keychain_store(&new_key);
    }

    Ok(())
}

/// Disable the vault: decrypt all stored credentials back to plain text, drop
/// the vault config, clear the keychain, and lock. Requires the vault to be
/// unlocked (the key is needed to decrypt). Inverse of `vault_setup`.
#[tauri::command]
pub async fn vault_disable(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
) -> Result<(), String> {
    if !vault_is_initialized(&pool).await {
        return Ok(());
    }
    let key = vault.get_key().map_err(|_| "vault_locked".to_string())?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let rows: Vec<(String, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT id, password_enc, key_path_enc, passphrase_enc FROM profiles")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    let decrypt = |blob: Option<Vec<u8>>| -> Result<Option<Vec<u8>>, String> {
        match blob.filter(|b| !b.is_empty()) {
            Some(b) => crypto::decrypt(&b, &key).map(Some).map_err(|e| e.to_string()),
            None => Ok(None),
        }
    };

    for (id, pass, kpath, phrase) in rows {
        let dec_pass = decrypt(pass)?;
        let dec_kpath = decrypt(kpath)?;
        let dec_phrase = decrypt(phrase)?;

        sqlx::query(
            "UPDATE profiles SET password_enc = ?, key_path_enc = ?, passphrase_enc = ? WHERE id = ?",
        )
        .bind(&dec_pass)
        .bind(&dec_kpath)
        .bind(&dec_phrase)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query("DELETE FROM vault_config")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let _ = keychain_delete();
    vault.lock();
    Ok(())
}

#[tauri::command]
pub async fn vault_forget_device(
    vault: tauri::State<'_, VaultState>,
) -> Result<(), String> {
    let _ = keychain_delete();
    vault.lock();
    Ok(())
}
