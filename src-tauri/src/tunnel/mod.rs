mod socks5;

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
    pub tunnel_type: String,
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
    #[serde(default = "default_tunnel_type")]
    pub tunnel_type: String,
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

fn default_tunnel_type() -> String {
    "local".to_string()
}

struct ActiveTunnel {
    /// Listener task for local/dynamic tunnels; None for remote forwards.
    abort: Option<JoinHandle<()>>,
    /// Set for remote (`-R`) tunnels, used to cancel the forward on stop.
    remote_port: Option<u16>,
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

        let (abort, remote_port_field): (Option<JoinHandle<()>>, Option<u16>) =
            if cfg.tunnel_type == "remote" {
                // Server binds the port; the SSH handler routes connections back
                // to the local target. No local listener.
                sessions
                    .add_remote_forward(
                        &session_id,
                        cfg.remote_port,
                        cfg.local_host.clone(),
                        cfg.local_port,
                    )
                    .await?;
                (None, Some(cfg.remote_port))
            } else {
                // Bind first so a port conflict surfaces synchronously.
                let listener = TcpListener::bind((cfg.local_host.as_str(), cfg.local_port))
                    .await
                    .map_err(|e| {
                        anyhow!("Failed to bind {}:{}: {}", cfg.local_host, cfg.local_port, e)
                    })?;

                let remote_host = cfg.remote_host.clone();
                let remote_port = cfg.remote_port;
                let is_dynamic = cfg.tunnel_type == "dynamic";
                let sid = session_id.clone();

                let handle = tokio::spawn(async move {
                    loop {
                        match listener.accept().await {
                            Ok((mut tcp, _addr)) => {
                                let sessions = sessions.clone();
                                let sid = sid.clone();
                                let rhost = remote_host.clone();
                                tokio::spawn(async move {
                                    if is_dynamic {
                                        // SOCKS5: resolve the target from the handshake.
                                        let (host, port) = match socks5::handshake(&mut tcp).await {
                                            Ok(t) => t,
                                            Err(e) => {
                                                log::warn!("socks5 handshake failed: {}", e);
                                                return;
                                            }
                                        };
                                        match sessions.open_direct_tcpip(&sid, &host, port).await {
                                            Ok(channel) => {
                                                if socks5::reply_success(&mut tcp).await.is_err() {
                                                    return;
                                                }
                                                let mut stream = channel.into_stream();
                                                let _ = tokio::io::copy_bidirectional(
                                                    &mut tcp,
                                                    &mut stream,
                                                )
                                                .await;
                                            }
                                            Err(e) => {
                                                log::warn!("socks5 target failed: {}", e);
                                                let _ = socks5::reply_failure(&mut tcp).await;
                                            }
                                        }
                                    } else {
                                        // Local forward: fixed remote target.
                                        match sessions
                                            .open_direct_tcpip(&sid, &rhost, remote_port)
                                            .await
                                        {
                                            Ok(channel) => {
                                                let mut stream = channel.into_stream();
                                                let _ = tokio::io::copy_bidirectional(
                                                    &mut tcp,
                                                    &mut stream,
                                                )
                                                .await;
                                            }
                                            Err(e) => {
                                                log::warn!("tunnel direct-tcpip failed: {}", e)
                                            }
                                        }
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
                (Some(handle), None)
            };

        let info = ActiveTunnelInfo {
            tunnel_id: cfg.tunnel_id.clone(),
            session_id: session_id.clone(),
            tunnel_type: cfg.tunnel_type,
            local_host: cfg.local_host,
            local_port: cfg.local_port,
            remote_host: cfg.remote_host,
            remote_port: cfg.remote_port,
            status: TunnelStatus::Active,
        };
        emit_status(&app, &k, &info);
        self.active.lock().await.insert(
            k,
            ActiveTunnel {
                abort,
                remote_port: remote_port_field,
                info,
            },
        );
        Ok(())
    }

    pub async fn stop(
        &self,
        session_id: &str,
        tunnel_id: &str,
        sessions: &SessionManager,
        app: &AppHandle,
    ) {
        let k = key(session_id, tunnel_id);
        if let Some(t) = self.active.lock().await.remove(&k) {
            if let Some(handle) = t.abort {
                handle.abort();
            }
            if let Some(rp) = t.remote_port {
                sessions.remove_remote_forward(session_id, rp).await;
            }
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
                if let Some(handle) = t.abort {
                    handle.abort();
                }
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
    sessions: tauri::State<'_, SessionManager>,
    manager: tauri::State<'_, TunnelManager>,
    app: AppHandle,
) -> CmdResult<()> {
    manager.stop(&session_id, &tunnel_id, sessions.inner(), &app).await;
    Ok(())
}

#[tauri::command]
pub async fn tunnel_active(
    session_id: String,
    manager: tauri::State<'_, TunnelManager>,
) -> CmdResult<Vec<ActiveTunnelInfo>> {
    Ok(manager.active_for(&session_id).await)
}
