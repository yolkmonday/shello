use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Context, Result};
use russh::client;
use russh::keys::key::PrivateKeyWithHashAlg;
use russh::ChannelMsg;
use log::warn;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use russh::keys::known_hosts::check_known_hosts;
use russh::keys::HashAlg;

use super::types::{AuthMethod, ConnectionConfig};

/// Remote-forward routing table: server bind port → local target (host, port).
pub(crate) type Forwards = Arc<Mutex<HashMap<u16, (String, u16)>>>;

/// Remove a host's entries from `~/.ssh/known_hosts` (used to recover from a
/// changed-key rejection). Only plain (non-hashed) entries are matched.
pub fn forget_host_key(host: &str, port: u16) -> Result<()> {
    let path = dirs::home_dir()
        .context("no home directory")?
        .join(".ssh")
        .join("known_hosts");
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&path)?;
    let needle_port = format!("[{}]:{}", host, port);
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return true;
            }
            let first = trimmed.split_whitespace().next().unwrap_or("");
            !first.split(',').any(|h| h == host || h == needle_port)
        })
        .collect();
    std::fs::write(&path, format!("{}\n", kept.join("\n")))?;
    Ok(())
}

/// An unknown server key awaiting the user's confirmation.
#[derive(Clone)]
pub(crate) struct PendingHostKey {
    pub key: russh::keys::PublicKey,
    pub fingerprint: String,
}

pub(crate) type PendingSlot = Arc<Mutex<Option<PendingHostKey>>>;

// ── SSH Event Handler ────────────────────────────────────────────────

pub(crate) struct SshHandler {
    forwards: Forwards,
    host: String,
    port: u16,
    pending: PendingSlot,
}

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        match check_known_hosts(&self.host, self.port, server_public_key) {
            // Known host, key matches.
            Ok(true) => Ok(true),
            // Unknown host: defer to a post-connect confirmation prompt.
            Ok(false) => {
                let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
                *self.pending.lock().await = Some(PendingHostKey {
                    key: server_public_key.clone(),
                    fingerprint,
                });
                Ok(true)
            }
            // Recorded key no longer matches — refuse (possible MITM).
            Err(russh::keys::Error::KeyChanged { .. }) => Err(anyhow::anyhow!(
                "host_key_changed: SSH host key for {} has changed — possible man-in-the-middle attack. Connection refused.",
                self.host
            )),
            // Other errors (e.g. missing known_hosts): treat as a new host.
            Err(e) => {
                warn!("known_hosts check failed ({}); treating {} as a new host", e, self.host);
                let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
                *self.pending.lock().await = Some(PendingHostKey {
                    key: server_public_key.clone(),
                    fingerprint,
                });
                Ok(true)
            }
        }
    }

    /// A connection arrived on a server-side `-R` forward. Route it to the
    /// configured local target.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        _connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let target = self.forwards.lock().await.get(&(connected_port as u16)).cloned();
        match target {
            Some((host, port)) => {
                tokio::spawn(async move {
                    match TcpStream::connect((host.as_str(), port)).await {
                        Ok(mut tcp) => {
                            let mut stream = channel.into_stream();
                            let _ = copy_bidirectional(&mut tcp, &mut stream).await;
                        }
                        Err(e) => {
                            log::warn!("remote-forward: connect {}:{} failed: {}", host, port, e)
                        }
                    }
                });
            }
            None => log::warn!("remote-forward: no route for port {}", connected_port),
        }
        Ok(())
    }
}

// ── SSH Client ───────────────────────────────────────────────────────

pub struct SshClient {
    handle: client::Handle<SshHandler>,
    forwards: Forwards,
    pending: PendingSlot,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected_at: String,
}

impl SshClient {
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        let russh_config = Arc::new(client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(300)),
            ..Default::default()
        });

        let forwards: Forwards = Arc::new(Mutex::new(HashMap::new()));
        let pending: PendingSlot = Arc::new(Mutex::new(None));
        let handler = SshHandler {
            forwards: forwards.clone(),
            host: config.host.clone(),
            port: config.port,
            pending: pending.clone(),
        };
        let timeout_secs = config.timeout_secs;
        let mut handle = timeout(
            Duration::from_secs(timeout_secs),
            client::connect(
                russh_config,
                (config.host.as_str(), config.port),
                handler,
            ),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Connection timed out after {} seconds", timeout_secs))?
        .context(format!("Failed to connect to {}:{}", config.host, config.port))?;

        // Authenticate
        let auth_result = match &config.auth {
            AuthMethod::Password { password } => {
                handle
                    .authenticate_password(&config.username, password)
                    .await
                    .context("Password authentication failed")?
            }
            AuthMethod::Key { private_key_path, passphrase } => {
                let private_key = russh::keys::load_secret_key(
                    private_key_path,
                    passphrase.as_deref(),
                )
                .context(format!("Failed to load private key from: {}", private_key_path))?;

                let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(private_key), None);

                handle
                    .authenticate_publickey(&config.username, key_with_alg)
                    .await
                    .context("Public key authentication failed")?
            }
        };

        if !matches!(auth_result, client::AuthResult::Success) {
            anyhow::bail!("Authentication failed for user '{}'", config.username);
        }

        let connected_at = chrono::Utc::now().to_rfc3339();

        Ok(Self {
            handle,
            forwards,
            pending,
            host: config.host,
            port: config.port,
            username: config.username,
            connected_at,
        })
    }

    pub fn handle(&self) -> &client::Handle<SshHandler> {
        &self.handle
    }

    /// The remote-forward routing table, shared with the SSH event handler.
    pub fn forwards(&self) -> Forwards {
        self.forwards.clone()
    }

    /// The slot holding an unverified host key awaiting user confirmation.
    pub fn pending(&self) -> PendingSlot {
        self.pending.clone()
    }

    pub async fn exec(&mut self, command: &str) -> Result<String> {
        let mut channel = self.handle.channel_open_session().await
            .context("Failed to open SSH channel")?;

        channel.exec(true, command).await
            .context(format!("Failed to execute command: {}", command))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    stdout.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { data, ext: 1 }) => {
                    stderr.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { .. }) => {}
                Some(ChannelMsg::Eof) => {}
                Some(ChannelMsg::Close) => break,
                None => break,
                _ => {}
            }
        }

        let mut output = String::from_utf8_lossy(&stdout).to_string();
        if !stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&String::from_utf8_lossy(&stderr));
        }

        Ok(output)
    }

    pub async fn close(self) -> Result<()> {
        self.handle
            .disconnect(russh::Disconnect::ByApplication, "bye", "en")
            .await
            .context("Failed to disconnect SSH session")?;
        Ok(())
    }
}
