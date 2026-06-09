use anyhow::{Context, Result};
use russh::ChannelMsg;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::io::AsyncWriteExt;
use log::warn;

use super::client::SshHandler;
use super::types::{SshDataEvent, SshClosedEvent};

/// An active PTY session with background I/O streaming.
pub struct PtySession {
    write_tx: mpsc::Sender<Vec<u8>>,
    resize_tx: mpsc::Sender<(u32, u32)>,
    log_tx: mpsc::Sender<LogCommand>,
}

enum LogCommand {
    Start(String), // file path
    Stop,
}

impl PtySession {
    /// Open a PTY session on an authenticated SSH connection.
    pub async fn open(
        handle: &russh::client::Handle<SshHandler>,
        session_id: String,
        cols: u32,
        rows: u32,
        app_handle: tauri::AppHandle,
    ) -> Result<Self> {
        let mut channel = handle.channel_open_session().await
            .context("Failed to open session channel for PTY")?;

        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .context("Failed to request PTY")?;

        channel
            .request_shell(true)
            .await
            .context("Failed to request shell")?;

        let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(64);
        let (resize_tx, mut resize_rx) = mpsc::channel::<(u32, u32)>(8);
        let (log_tx, mut log_rx) = mpsc::channel::<LogCommand>(4);

        let sid = session_id.clone();
        tokio::spawn(async move {
            let mut log_file: Option<tokio::fs::File> = None;

            loop {
                tokio::select! {
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                let bytes = data.to_vec();

                                // Write to log file if enabled
                                if let Some(ref mut f) = log_file {
                                    let _ = f.write_all(&bytes).await;
                                }

                                let _ = app_handle.emit("ssh_data", SshDataEvent {
                                    session_id: sid.clone(),
                                    data: bytes,
                                });
                            }
                            Some(ChannelMsg::Eof) => {}
                            Some(ChannelMsg::Close) | None => {
                                let _ = app_handle.emit("ssh_closed", SshClosedEvent {
                                    session_id: sid.clone(),
                                    reason: "Connection closed".to_string(),
                                });
                                break;
                            }
                            _ => {}
                        }
                    }
                    data = write_rx.recv() => {
                        match data {
                            Some(bytes) => {
                                if let Err(e) = channel.data(&bytes[..]).await {
                                    warn!("Failed to write to PTY: {}", e);
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    size = resize_rx.recv() => {
                        match size {
                            Some((cols, rows)) => {
                                if let Err(e) = channel.window_change(cols, rows, 0, 0).await {
                                    warn!("Failed to resize PTY: {}", e);
                                }
                            }
                            None => break,
                        }
                    }
                    cmd = log_rx.recv() => {
                        match cmd {
                            Some(LogCommand::Start(path)) => {
                                match tokio::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(&path)
                                    .await
                                {
                                    Ok(f) => { log_file = Some(f); }
                                    Err(e) => { warn!("Failed to open log file {}: {}", path, e); }
                                }
                            }
                            Some(LogCommand::Stop) => {
                                log_file = None;
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Ok(Self { write_tx, resize_tx, log_tx })
    }

    pub async fn write(&self, data: Vec<u8>) -> Result<()> {
        self.write_tx
            .send(data)
            .await
            .map_err(|_| anyhow::anyhow!("PTY session closed"))
    }

    pub async fn resize(&self, cols: u32, rows: u32) -> Result<()> {
        self.resize_tx
            .send((cols, rows))
            .await
            .map_err(|_| anyhow::anyhow!("PTY session closed"))
    }

    pub async fn start_logging(&self, path: String) -> Result<()> {
        self.log_tx
            .send(LogCommand::Start(path))
            .await
            .map_err(|_| anyhow::anyhow!("PTY session closed"))
    }

    pub async fn stop_logging(&self) -> Result<()> {
        self.log_tx
            .send(LogCommand::Stop)
            .await
            .map_err(|_| anyhow::anyhow!("PTY session closed"))
    }
}
