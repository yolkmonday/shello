use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::Mutex;
use tauri::Emitter;
use log::warn;

use crate::ssh::types::{SshDataEvent, SshClosedEvent};

struct LocalSession {
    writer: Box<dyn std::io::Write + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
}

#[derive(Clone)]
pub struct LocalSessionManager {
    sessions: Arc<Mutex<HashMap<String, LocalSession>>>,
}

impl LocalSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn open(
        &self,
        cols: u32,
        rows: u32,
        app_handle: tauri::AppHandle,
    ) -> Result<String> {
        let session_id = ulid::Ulid::new().to_string();
        let sid = session_id.clone();

        let pty_system = native_pty_system();
        let pty = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to allocate PTY")?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "powershell".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));

        let child = pty
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;

        let reader = pty
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let writer = pty
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        let master = Arc::new(Mutex::new(pty.master));

        // Background task: read PTY output and emit to frontend
        let sid2 = sid.clone();
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let bytes = buf[..n].to_vec();
                        let _ = app_handle.emit("ssh_data", SshDataEvent {
                            session_id: sid2.clone(),
                            data: bytes,
                        });
                    }
                    Err(e) => {
                        warn!("Local PTY read error: {}", e);
                        break;
                    }
                }
            }
            let _ = app_handle.emit("ssh_closed", SshClosedEvent {
                session_id: sid2.clone(),
                reason: "Shell exited".to_string(),
            });
        });

        let session = LocalSession {
            writer,
            _child: child,
            master,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: String) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .context("Local session not found")?;
        use std::io::Write;
        session
            .writer
            .write_all(data.as_bytes())
            .context("Failed to write to local PTY")?;
        session.writer.flush().context("Failed to flush local PTY")?;
        Ok(())
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .context("Local session not found")?;
        let master = session.master.lock().await;
        master
            .resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize local PTY")?;
        Ok(())
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        sessions
            .remove(session_id)
            .context("Local session not found")?;
        Ok(())
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        let sessions = self.sessions.lock().await;
        sessions.contains_key(session_id)
    }
}
