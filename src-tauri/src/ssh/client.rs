use std::sync::Arc;
use anyhow::{Context, Result};
use russh::client;
use russh::keys::key::PrivateKeyWithHashAlg;
use russh::ChannelMsg;
use log::warn;
use tokio::time::{timeout, Duration};

use super::types::{AuthMethod, ConnectionConfig};

// ── SSH Event Handler ────────────────────────────────────────────────

pub(crate) struct SshHandler;

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        warn!("Accepting server key (TOFU)");
        Ok(true)
    }
}

// ── SSH Client ───────────────────────────────────────────────────────

pub struct SshClient {
    handle: client::Handle<SshHandler>,
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

        let handler = SshHandler;
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
            host: config.host,
            port: config.port,
            username: config.username,
            connected_at,
        })
    }

    pub fn handle(&self) -> &client::Handle<SshHandler> {
        &self.handle
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
