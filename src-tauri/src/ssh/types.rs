use serde::{Deserialize, Serialize};

/// Configuration for establishing an SSH connection.
/// Received from the Vue frontend via Tauri invoke().
#[derive(Debug, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    10
}

/// How to authenticate with the SSH server.
/// Uses serde's tagged enum: the "type" field determines which variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    #[serde(rename = "password")]
    Password { password: String },
    #[serde(rename = "key")]
    Key {
        private_key_path: String,
        passphrase: Option<String>,
    },
}

/// Info about an active session, returned to the frontend.
/// Output-only (Rust -> Vue), so only Serialize is needed.
#[derive(Debug, Serialize, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected_at: String,
}

/// Detected OS information from a remote server.
#[derive(Debug, Serialize, Clone)]
pub struct OsInfo {
    /// General OS type: "linux", "macos", "windows", "freebsd", "unknown"
    pub os_type: String,
    /// Specific distribution/version: "ubuntu", "debian", "centos", "alpine", etc.
    pub distro: String,
}

/// Parse OS info from /etc/os-release content and uname output.
pub fn parse_os_info(os_release: &str, uname: &str) -> (String, String) {
    let uname_trimmed = uname.trim().to_lowercase();

    // Try to parse /etc/os-release for Linux distros
    if !os_release.is_empty() {
        let id = os_release
            .lines()
            .find(|l| l.starts_with("ID="))
            .map(|l| l.trim_start_matches("ID=").trim_matches('"').to_lowercase())
            .unwrap_or_default();

        if !id.is_empty() {
            let distro = match id.as_str() {
                "ubuntu" => "ubuntu",
                "debian" => "debian",
                "centos" => "centos",
                "rhel" | "redhat" => "redhat",
                "fedora" => "fedora",
                "arch" | "archarm" => "archlinux",
                "alpine" => "alpine",
                "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => "opensuse",
                "sles" | "suse" => "suse",
                "rocky" => "rocky",
                "almalinux" | "alma" => "almalinux",
                "gentoo" => "gentoo",
                "manjaro" => "manjaro",
                "kali" => "kali",
                "pop" => "pop_os",
                "mint" | "linuxmint" => "linuxmint",
                "void" => "void",
                "nixos" => "nixos",
                "amzn" => "amazon",
                "ol" | "oracle" => "oracle",
                _ => &id,
            };
            return ("linux".to_string(), distro.to_string());
        }
    }

    // Fallback to uname
    match uname_trimmed.as_str() {
        "linux" => ("linux".to_string(), "linux".to_string()),
        "darwin" => ("macos".to_string(), "macos".to_string()),
        "freebsd" => ("freebsd".to_string(), "freebsd".to_string()),
        "openbsd" => ("openbsd".to_string(), "openbsd".to_string()),
        "netbsd" => ("netbsd".to_string(), "netbsd".to_string()),
        s if s.starts_with("mingw") || s.starts_with("msys") || s.starts_with("cygwin") => {
            ("windows".to_string(), "windows".to_string())
        }
        _ => ("unknown".to_string(), "unknown".to_string()),
    }
}

/// Event payload sent from Rust to frontend when SSH data arrives.
/// Emitted as Tauri event "ssh_data".
#[derive(Clone, Serialize)]
pub struct SshDataEvent {
    pub session_id: String,
    pub data: Vec<u8>,
}

/// Event payload sent when an SSH session closes.
/// Emitted as Tauri event "ssh_closed".
#[derive(Clone, Serialize)]
pub struct SshClosedEvent {
    pub session_id: String,
    pub reason: String,
}
