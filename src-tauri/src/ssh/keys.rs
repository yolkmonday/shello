use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct SshKeyInfo {
    pub name: String,
    pub path: String,
    pub key_type: String,
    pub has_public: bool,
    pub public_key: Option<String>,
}

pub fn get_ssh_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".ssh")
}

pub fn list_keys() -> Result<Vec<SshKeyInfo>> {
    let ssh_dir = get_ssh_dir();
    if !ssh_dir.exists() {
        return Ok(vec![]);
    }

    let mut keys = Vec::new();
    let private_key_extensions = ["", "pem"];
    let skip_files = ["known_hosts", "known_hosts.old", "authorized_keys", "config", "environment"];

    for entry in std::fs::read_dir(&ssh_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        // Skip pub keys, known_hosts, config, etc.
        if name.ends_with(".pub") || skip_files.contains(&name.as_str()) {
            continue;
        }

        // Check if it looks like a private key
        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        if !ext.is_empty() && !private_key_extensions.contains(&ext.as_str()) {
            continue;
        }

        // Try to read first line to detect key type
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let first_line = content.lines().next().unwrap_or("");

        let key_type = if first_line.contains("OPENSSH PRIVATE KEY") {
            detect_openssh_key_type(&content)
        } else if first_line.contains("RSA PRIVATE KEY") {
            "RSA".to_string()
        } else if first_line.contains("EC PRIVATE KEY") {
            "ECDSA".to_string()
        } else if first_line.contains("DSA PRIVATE KEY") {
            "DSA".to_string()
        } else if first_line.contains("PRIVATE KEY") {
            "Unknown".to_string()
        } else {
            continue; // Not a key file
        };

        // Check for corresponding .pub file
        let pub_path = path.with_extension(format!("{}.pub", ext).trim_start_matches('.'));
        let pub_path2 = PathBuf::from(format!("{}.pub", path.display()));
        let pub_file = if pub_path.exists() {
            Some(pub_path)
        } else if pub_path2.exists() {
            Some(pub_path2)
        } else {
            None
        };

        let public_key = pub_file.and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.trim().to_string());

        keys.push(SshKeyInfo {
            name,
            path: path.to_string_lossy().to_string(),
            key_type,
            has_public: public_key.is_some(),
            public_key,
        });
    }

    keys.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(keys)
}

fn detect_openssh_key_type(content: &str) -> String {
    // OpenSSH format doesn't clearly indicate key type in the PEM header.
    // We check the corresponding .pub file or default to "Ed25519"
    if content.contains("ssh-ed25519") {
        "Ed25519".to_string()
    } else if content.contains("ssh-rsa") {
        "RSA".to_string()
    } else if content.contains("ecdsa") {
        "ECDSA".to_string()
    } else {
        "Ed25519".to_string()
    }
}

pub async fn generate_key(name: &str, key_type: &str, passphrase: Option<&str>) -> Result<SshKeyInfo> {
    let ssh_dir = get_ssh_dir();
    std::fs::create_dir_all(&ssh_dir)?;

    let key_path = ssh_dir.join(name);
    if key_path.exists() {
        anyhow::bail!("Key file already exists: {}", name);
    }

    let kt = match key_type {
        "ed25519" => "ed25519",
        "rsa" => "rsa",
        "ecdsa" => "ecdsa",
        _ => "ed25519",
    };

    let mut cmd = tokio::process::Command::new("ssh-keygen");
    cmd.arg("-t").arg(kt)
       .arg("-f").arg(&key_path)
       .arg("-q");

    match passphrase {
        Some(p) if !p.is_empty() => { cmd.arg("-N").arg(p); }
        _ => { cmd.arg("-N").arg(""); }
    }

    if kt == "rsa" {
        cmd.arg("-b").arg("4096");
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ssh-keygen failed: {}", stderr);
    }

    let pub_path = PathBuf::from(format!("{}.pub", key_path.display()));
    let public_key = std::fs::read_to_string(&pub_path).ok()
        .map(|s| s.trim().to_string());

    Ok(SshKeyInfo {
        name: name.to_string(),
        path: key_path.to_string_lossy().to_string(),
        key_type: kt.to_uppercase(),
        has_public: public_key.is_some(),
        public_key,
    })
}
