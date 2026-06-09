use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::sync::Mutex;

use super::client::SshClient;
use super::pty::PtySession;
use super::types::{ConnectionConfig, OsInfo, SessionInfo, parse_os_info};

pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, SshClient>>>,
    pty_sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pty_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&self, config: ConnectionConfig) -> Result<String> {
        let client = SshClient::connect(config).await?;
        let session_id = ulid::Ulid::new().to_string();
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), client);
        Ok(session_id)
    }

    /// Open a PTY on an existing session.
    /// Note: russh Handle does NOT implement Clone, so we hold the
    /// sessions lock during PTY open. This blocks other session
    /// operations briefly during the PTY handshake (typically <100ms).
    pub async fn open_pty(
        &self,
        session_id: &str,
        cols: u32,
        rows: u32,
        app_handle: tauri::AppHandle,
    ) -> Result<()> {
        let sessions = self.sessions.lock().await;
        let client = sessions
            .get(session_id)
            .context(format!("Session not found: {}", session_id))?;

        let pty = PtySession::open(
            client.handle(),
            session_id.to_string(),
            cols,
            rows,
            app_handle,
        )
        .await?;
        drop(sessions);

        let mut pty_sessions = self.pty_sessions.lock().await;
        pty_sessions.insert(session_id.to_string(), pty);
        Ok(())
    }

    pub async fn write(&self, session_id: &str, data: String) -> Result<()> {
        let pty_sessions = self.pty_sessions.lock().await;
        let pty = pty_sessions
            .get(session_id)
            .context(format!("PTY session not found: {}", session_id))?;
        pty.write(data.into_bytes()).await
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<()> {
        let pty_sessions = self.pty_sessions.lock().await;
        let pty = pty_sessions
            .get(session_id)
            .context(format!("PTY session not found: {}", session_id))?;
        pty.resize(cols, rows).await
    }

    pub async fn exec(&self, session_id: &str, command: &str) -> Result<String> {
        let mut sessions = self.sessions.lock().await;
        let client = sessions
            .get_mut(session_id)
            .context(format!("Session not found: {}", session_id))?;
        client.exec(command).await
    }

    pub async fn detect_os(&self, session_id: &str) -> Result<OsInfo> {
        let mut sessions = self.sessions.lock().await;
        let client = sessions
            .get_mut(session_id)
            .context(format!("Session not found: {}", session_id))?;

        // Try to get OS info from /etc/os-release first (Linux distros)
        let os_release = client.exec("cat /etc/os-release 2>/dev/null").await.unwrap_or_default();

        // Also get uname as fallback
        let uname = client.exec("uname -s 2>/dev/null").await.unwrap_or_default();

        let (os_type, distro) = parse_os_info(&os_release, &uname);

        Ok(OsInfo { os_type, distro })
    }

    pub async fn start_logging(&self, session_id: &str, path: &str) -> Result<()> {
        let pty_sessions = self.pty_sessions.lock().await;
        let pty = pty_sessions
            .get(session_id)
            .context(format!("PTY session not found: {}", session_id))?;
        pty.start_logging(path.to_string()).await
    }

    pub async fn stop_logging(&self, session_id: &str) -> Result<()> {
        let pty_sessions = self.pty_sessions.lock().await;
        let pty = pty_sessions
            .get(session_id)
            .context(format!("PTY session not found: {}", session_id))?;
        pty.stop_logging().await
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<()> {
        {
            let mut pty_sessions = self.pty_sessions.lock().await;
            pty_sessions.remove(session_id);
        }
        let mut sessions = self.sessions.lock().await;
        let client = sessions
            .remove(session_id)
            .context(format!("Session not found: {}", session_id))?;
        client.close().await
    }

    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.lock().await;
        sessions
            .iter()
            .map(|(id, client)| SessionInfo {
                id: id.clone(),
                host: client.host.clone(),
                port: client.port,
                username: client.username.clone(),
                connected_at: client.connected_at.clone(),
            })
            .collect()
    }
}
