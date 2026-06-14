use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::db::tunnels::{
    self, CreateTunnelInput, Tunnel, UpdateTunnelInput,
};
use crate::db::DbPool;
use crate::ssh::session::SessionManager;

/// Runtime status of an active tunnel, serialized as `{ state, message? }`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum TunnelStatus {
    Active,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveTunnelInfo {
    pub tunnel_id: String,
    pub session_id: String,
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub status: TunnelStatus,
}

/// A request to start a local forward (from a saved tunnel or an ad-hoc form).
#[derive(Debug, Clone, Deserialize)]
pub struct StartTunnelConfig {
    pub tunnel_id: String,
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

struct ActiveTunnel {
    abort: JoinHandle<()>,
    info: ActiveTunnelInfo,
}

/// Manages running local-forward listeners, keyed by `"{session}:{tunnel}"`.
pub struct TunnelManager {
    active: Arc<Mutex<HashMap<String, ActiveTunnel>>>,
}

fn key(session_id: &str, tunnel_id: &str) -> String {
    format!("{session_id}:{tunnel_id}")
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(
        &self,
        session_id: String,
        cfg: StartTunnelConfig,
        sessions: SessionManager,
        app: AppHandle,
    ) -> Result<()> {
        let k = key(&session_id, &cfg.tunnel_id);

        // Bind first so a port conflict surfaces synchronously to the caller.
        let listener = TcpListener::bind((cfg.local_host.as_str(), cfg.local_port))
            .await
            .map_err(|e| anyhow!("Failed to bind {}:{}: {}", cfg.local_host, cfg.local_port, e))?;

        let remote_host = cfg.remote_host.clone();
        let remote_port = cfg.remote_port;
        let sid = session_id.clone();

        let abort = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut tcp, _addr)) => {
                        let sessions = sessions.clone();
                        let sid = sid.clone();
                        let rhost = remote_host.clone();
                        tokio::spawn(async move {
                            match sessions.open_direct_tcpip(&sid, &rhost, remote_port).await {
                                Ok(channel) => {
                                    let mut stream = channel.into_stream();
                                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                                }
                                Err(e) => log::warn!("tunnel direct-tcpip failed: {}", e),
                            }
                        });
                    }
                    Err(e) => {
                        log::warn!("tunnel accept error: {}", e);
                        break;
                    }
                }
            }
        });

        let info = ActiveTunnelInfo {
            tunnel_id: cfg.tunnel_id.clone(),
            session_id: session_id.clone(),
            local_host: cfg.local_host,
            local_port: cfg.local_port,
            remote_host: cfg.remote_host,
            remote_port: cfg.remote_port,
            status: TunnelStatus::Active,
        };
        emit_status(&app, &k, &info);
        self.active.lock().await.insert(k, ActiveTunnel { abort, info });
        Ok(())
    }

    pub async fn stop(&self, session_id: &str, tunnel_id: &str, app: &AppHandle) {
        let k = key(session_id, tunnel_id);
        if let Some(t) = self.active.lock().await.remove(&k) {
            t.abort.abort();
            let mut info = t.info;
            info.status = TunnelStatus::Stopped;
            emit_status(app, &k, &info);
        }
    }

    pub async fn stop_all(&self, session_id: &str) {
        let prefix = format!("{session_id}:");
        let mut active = self.active.lock().await;
        let keys: Vec<String> = active
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        for k in keys {
            if let Some(t) = active.remove(&k) {
                t.abort.abort();
            }
        }
    }

    pub async fn active_for(&self, session_id: &str) -> Vec<ActiveTunnelInfo> {
        let prefix = format!("{session_id}:");
        self.active
            .lock()
            .await
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, t)| t.info.clone())
            .collect()
    }
}

fn emit_status(app: &AppHandle, k: &str, info: &ActiveTunnelInfo) {
    let _ = app.emit(
        "tunnel-status",
        serde_json::json!({ "key": k, "info": info }),
    );
}

// ── Commands ─────────────────────────────────────────────────────────

type CmdResult<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn tunnel_list(pool: tauri::State<'_, DbPool>, profile_id: String) -> CmdResult<Vec<Tunnel>> {
    tunnels::list_tunnels(&pool, &profile_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tunnel_create(pool: tauri::State<'_, DbPool>, input: CreateTunnelInput) -> CmdResult<Tunnel> {
    tunnels::create_tunnel(&pool, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tunnel_update(pool: tauri::State<'_, DbPool>, id: String, input: UpdateTunnelInput) -> CmdResult<Tunnel> {
    tunnels::update_tunnel(&pool, &id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tunnel_delete(pool: tauri::State<'_, DbPool>, id: String) -> CmdResult<()> {
    tunnels::delete_tunnel(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tunnel_start(
    session_id: String,
    config: StartTunnelConfig,
    sessions: tauri::State<'_, SessionManager>,
    manager: tauri::State<'_, TunnelManager>,
    app: AppHandle,
) -> CmdResult<()> {
    manager
        .start(session_id, config, sessions.inner().clone(), app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tunnel_stop(
    session_id: String,
    tunnel_id: String,
    manager: tauri::State<'_, TunnelManager>,
    app: AppHandle,
) -> CmdResult<()> {
    manager.stop(&session_id, &tunnel_id, &app).await;
    Ok(())
}

#[tauri::command]
pub async fn tunnel_active(
    session_id: String,
    manager: tauri::State<'_, TunnelManager>,
) -> CmdResult<Vec<ActiveTunnelInfo>> {
    Ok(manager.active_for(&session_id).await)
}
