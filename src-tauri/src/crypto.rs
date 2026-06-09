use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

const PBKDF2_ITERATIONS: u32 = 600_000;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Generate a 32-byte random salt.
pub fn generate_salt() -> [u8; KEY_LEN] {
    let mut salt = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 256-bit key from password + salt using PBKDF2-HMAC-SHA256.
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// Encrypt plaintext with AES-256-GCM.
/// Returns: nonce (12 bytes) || ciphertext || tag (16 bytes).
pub fn encrypt(plaintext: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // nonce || ciphertext (which includes the tag appended by aes-gcm)
    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// Decrypt blob (nonce || ciphertext || tag) with AES-256-GCM.
pub fn decrypt(blob: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN + 16 {
        return Err(anyhow!("Blob too short to contain nonce + tag"));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let nonce = Nonce::from_slice(&blob[..NONCE_LEN]);
    let ciphertext = &blob[NONCE_LEN..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed (wrong key?): {}", e))
}
