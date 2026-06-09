pub mod types;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileAttributes;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::ssh::session::SessionManager;
use types::{join_remote, FileEntry};

const CHUNK: usize = 32 * 1024;
const EMIT_EVERY: u64 = 128 * 1024;

/// Manages one persistent SFTP session per SSH connection plus active transfer
/// cancellation flags.
pub struct SftpManager {
    sessions: Arc<Mutex<HashMap<String, Arc<SftpSession>>>>,
    cancels: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl SftpManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            cancels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Open an SFTP session over an already-prepared channel (subsystem
    /// requested by the caller). Returns the remote home (realpath ".").
    pub async fn open(
        &self,
        session_id: String,
        channel: russh::Channel<russh::client::Msg>,
    ) -> Result<String> {
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| anyhow!("sftp init failed: {e}"))?;
        let home = sftp
            .canonicalize(".")
            .await
            .unwrap_or_else(|_| "/".to_string());
        self.sessions
            .lock()
            .await
            .insert(session_id, Arc::new(sftp));
        Ok(home)
    }

    async fn session(&self, id: &str) -> Result<Arc<SftpSession>> {
        self.sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .context("SFTP session not open")
    }

    pub async fn list(&self, id: &str, path: &str) -> Result<Vec<FileEntry>> {
        let sftp = self.session(id).await?;
        let read_dir = sftp
            .read_dir(path)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        let mut out = Vec::new();
        for entry in read_dir {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let ft = entry.file_type();
            let meta = entry.metadata();
            out.push(FileEntry {
                path: join_remote(path, &name),
                name,
                is_dir: ft.is_dir(),
                is_symlink: ft.is_symlink(),
                size: meta.size.unwrap_or(0),
                modified: meta.mtime.map(|m| m as i64),
                mode: meta.permissions,
            });
        }
        Ok(out)
    }

    pub async fn mkdir(&self, id: &str, path: &str) -> Result<()> {
        let sftp = self.session(id).await?;
        sftp.create_dir(path).await.map_err(|e| anyhow!("{e}"))
    }

    pub async fn delete(&self, id: &str, path: &str, is_dir: bool) -> Result<()> {
        let sftp = self.session(id).await?;
        if is_dir {
            sftp.remove_dir(path).await.map_err(|e| anyhow!("{e}"))
        } else {
            sftp.remove_file(path).await.map_err(|e| anyhow!("{e}"))
        }
    }

    pub async fn rename(&self, id: &str, from: &str, to: &str) -> Result<()> {
        let sftp = self.session(id).await?;
        sftp.rename(from, to).await.map_err(|e| anyhow!("{e}"))
    }

    pub async fn create_file(&self, id: &str, path: &str) -> Result<()> {
        let sftp = self.session(id).await?;
        sftp.create(path).await.map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    pub async fn chmod(&self, id: &str, path: &str, mode: u32) -> Result<()> {
        let sftp = self.session(id).await?;
        let attrs = FileAttributes {
            permissions: Some(mode),
            ..Default::default()
        };
        sftp.set_metadata(path, attrs)
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn download(
        &self,
        id: &str,
        remote: &str,
        local: &str,
        transfer_id: String,
        app: AppHandle,
    ) -> Result<()> {
        let sftp = self.session(id).await?;
        let total = sftp
            .metadata(remote)
            .await
            .ok()
            .and_then(|m| m.size)
            .unwrap_or(0);
        let mut remote_file = sftp.open(remote).await.map_err(|e| anyhow!("{e}"))?;
        let cancel = self.register(&transfer_id).await;

        let result = async {
            let mut local_file = tokio::fs::File::create(local).await?;
            let mut buf = vec![0u8; CHUNK];
            let mut transferred: u64 = 0;
            let mut last_emit: u64 = 0;
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(anyhow!("cancelled"));
                }
                let n = remote_file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                local_file.write_all(&buf[..n]).await?;
                transferred += n as u64;
                if transferred - last_emit >= EMIT_EVERY {
                    last_emit = transferred;
                    emit_progress(&app, &transfer_id, transferred, total);
                }
            }
            local_file.flush().await?;
            emit_progress(&app, &transfer_id, transferred, total);
            Ok(())
        }
        .await;

        self.finish(&transfer_id).await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(local).await;
        }
        result
    }

    pub async fn upload(
        &self,
        id: &str,
        local: &str,
        remote: &str,
        transfer_id: String,
        app: AppHandle,
    ) -> Result<()> {
        let sftp = self.session(id).await?;
        let mut local_file = tokio::fs::File::open(local).await?;
        let total = local_file.metadata().await.map(|m| m.len()).unwrap_or(0);
        let mut remote_file = sftp.create(remote).await.map_err(|e| anyhow!("{e}"))?;
        let cancel = self.register(&transfer_id).await;

        let result = async {
            let mut buf = vec![0u8; CHUNK];
            let mut transferred: u64 = 0;
            let mut last_emit: u64 = 0;
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(anyhow!("cancelled"));
                }
                let n = local_file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                remote_file
                    .write_all(&buf[..n])
                    .await
                    .map_err(|e| anyhow!("{e}"))?;
                transferred += n as u64;
                if transferred - last_emit >= EMIT_EVERY {
                    last_emit = transferred;
                    emit_progress(&app, &transfer_id, transferred, total);
                }
            }
            remote_file.flush().await.map_err(|e| anyhow!("{e}"))?;
            remote_file.shutdown().await.map_err(|e| anyhow!("{e}"))?;
            emit_progress(&app, &transfer_id, transferred, total);
            Ok(())
        }
        .await;

        self.finish(&transfer_id).await;
        if result.is_err() {
            let _ = sftp.remove_file(remote).await;
        }
        result
    }

    pub async fn cancel(&self, transfer_id: &str) {
        if let Some(flag) = self.cancels.lock().await.get(transfer_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub async fn close(&self, id: &str) {
        self.sessions.lock().await.remove(id);
    }

    async fn register(&self, transfer_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .await
            .insert(transfer_id.to_string(), flag.clone());
        flag
    }

    async fn finish(&self, transfer_id: &str) {
        self.cancels.lock().await.remove(transfer_id);
    }
}

fn emit_progress(app: &AppHandle, transfer_id: &str, transferred: u64, total: u64) {
    let _ = app.emit(
        "sftp-progress",
        serde_json::json!({
            "transfer_id": transfer_id,
            "transferred": transferred,
            "total": total,
        }),
    );
}

/// List a local directory. Falls back to the user home if `path` is empty.
pub fn local_list(path: &str) -> Result<Vec<FileEntry>> {
    let dir = if path.is_empty() {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    } else {
        path.to_string()
    };

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            Some(meta.permissions().mode())
        };
        #[cfg(not(unix))]
        let mode = None;
        out.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
            size: meta.len(),
            modified,
            mode,
        });
    }
    Ok(out)
}

/// Return the local home directory as a starting path for the local pane.
pub fn local_home() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string())
}

// ── Tauri commands ───────────────────────────────────────────────────

type CmdResult<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn sftp_open(
    session_id: String,
    sessions: tauri::State<'_, SessionManager>,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<String> {
    let channel = sessions
        .open_sftp_channel(&session_id)
        .await
        .map_err(|e| e.to_string())?;
    sftp.open(session_id, channel)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_list(
    session_id: String,
    path: String,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<Vec<FileEntry>> {
    sftp.list(&session_id, &path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_local_list(path: String) -> CmdResult<Vec<FileEntry>> {
    local_list(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sftp_local_home() -> String {
    local_home()
}

#[tauri::command]
pub async fn sftp_mkdir(
    session_id: String,
    path: String,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<()> {
    sftp.mkdir(&session_id, &path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_delete(
    session_id: String,
    path: String,
    is_dir: bool,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<()> {
    sftp.delete(&session_id, &path, is_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_rename(
    session_id: String,
    from: String,
    to: String,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<()> {
    sftp.rename(&session_id, &from, &to)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_create_file(
    session_id: String,
    path: String,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<()> {
    sftp.create_file(&session_id, &path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_chmod(
    session_id: String,
    path: String,
    mode: u32,
    sftp: tauri::State<'_, SftpManager>,
) -> CmdResult<()> {
    sftp.chmod(&session_id, &path, mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_download(
    session_id: String,
    remote_path: String,
    local_path: String,
    transfer_id: String,
    sftp: tauri::State<'_, SftpManager>,
    app: AppHandle,
) -> CmdResult<()> {
    sftp.download(&session_id, &remote_path, &local_path, transfer_id, app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_upload(
    session_id: String,
    local_path: String,
    remote_path: String,
    transfer_id: String,
    sftp: tauri::State<'_, SftpManager>,
    app: AppHandle,
) -> CmdResult<()> {
    sftp.upload(&session_id, &local_path, &remote_path, transfer_id, app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_cancel(transfer_id: String, sftp: tauri::State<'_, SftpManager>) -> CmdResult<()> {
    sftp.cancel(&transfer_id).await;
    Ok(())
}

#[tauri::command]
pub async fn sftp_close(session_id: String, sftp: tauri::State<'_, SftpManager>) -> CmdResult<()> {
    sftp.close(&session_id).await;
    Ok(())
}
