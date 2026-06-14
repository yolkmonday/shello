use serde::Serialize;

/// A single concrete host parsed from `~/.ssh/config` (wildcard/pattern blocks
/// are excluded).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParsedHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
}

fn is_pattern(alias: &str) -> bool {
    alias.contains('*') || alias.contains('?') || alias.contains('!')
}

fn expand_home(path: &str, home: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{}", home.trim_end_matches('/'), rest)
    } else if path == "~" {
        home.to_string()
    } else {
        path.to_string()
    }
}

/// Split an ssh_config line into `(key, value)`. Supports `Key value` and
/// `Key=value` (with optional surrounding spaces around `=`).
fn split_kv(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
        i += 1;
    }
    let key = &line[..i];
    let mut rest = line[i..].trim_start();
    if let Some(stripped) = rest.strip_prefix('=') {
        rest = stripped.trim_start();
    }
    (key, rest)
}

/// Parse the text of an ssh_config into concrete hosts. `home` is used to
/// expand a leading `~` in `IdentityFile`.
pub fn parse_ssh_config(content: &str, home: &str) -> Vec<ParsedHost> {
    let mut hosts: Vec<ParsedHost> = Vec::new();
    // Indices into `hosts` for the aliases declared on the current `Host` line.
    let mut current: Vec<usize> = Vec::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = split_kv(line);
        if key.is_empty() {
            continue;
        }
        let key_lower = key.to_ascii_lowercase();

        if key_lower == "host" {
            current.clear();
            for alias in value.split_whitespace() {
                if is_pattern(alias) {
                    continue;
                }
                current.push(hosts.len());
                hosts.push(ParsedHost {
                    alias: alias.to_string(),
                    hostname: None,
                    user: None,
                    port: None,
                    identity_file: None,
                    proxy_jump: None,
                });
            }
            continue;
        }

        if current.is_empty() || value.is_empty() {
            continue;
        }

        for &idx in &current {
            let h = &mut hosts[idx];
            match key_lower.as_str() {
                "hostname" => h.hostname = Some(value.to_string()),
                "user" => h.user = Some(value.to_string()),
                "port" => h.port = value.parse().ok(),
                "identityfile" => h.identity_file = Some(expand_home(value, home)),
                "proxyjump" => h.proxy_jump = Some(value.to_string()),
                _ => {}
            }
        }
    }

    hosts
}

/// Read and parse the user's `~/.ssh/config`. A missing file yields an empty
/// list; unreadable lines are skipped by the parser.
#[tauri::command]
pub fn ssh_config_parse() -> Result<Vec<ParsedHost>, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let path = home.join(".ssh").join("config");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let home_str = home.to_string_lossy();
    Ok(parse_ssh_config(&content, &home_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_host() {
        let c = "Host web\n  HostName 1.2.3.4\n  User deploy\n  Port 2222\n";
        let h = parse_ssh_config(c, "/home/me");
        assert_eq!(h.len(), 1);
        assert_eq!(
            h[0],
            ParsedHost {
                alias: "web".into(),
                hostname: Some("1.2.3.4".into()),
                user: Some("deploy".into()),
                port: Some(2222),
                identity_file: None,
                proxy_jump: None,
            }
        );
    }

    #[test]
    fn identity_file_tilde_expanded() {
        let c = "Host k\n  IdentityFile ~/.ssh/id_ed25519\n";
        let h = parse_ssh_config(c, "/home/me");
        assert_eq!(h[0].identity_file, Some("/home/me/.ssh/id_ed25519".into()));
    }

    #[test]
    fn wildcard_skipped() {
        let c = "Host *\n  User all\nHost real\n  HostName r\n";
        let h = parse_ssh_config(c, "/h");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].alias, "real");
    }

    #[test]
    fn proxy_jump_recorded() {
        let c = "Host internal\n  HostName 10.0.0.1\n  ProxyJump bastion\n";
        let h = parse_ssh_config(c, "/h");
        assert_eq!(h[0].proxy_jump, Some("bastion".into()));
    }

    #[test]
    fn no_hostname_uses_alias() {
        let c = "Host myserver.com\n  User x\n";
        let h = parse_ssh_config(c, "/h");
        assert_eq!(h[0].hostname, None);
        assert_eq!(h[0].alias, "myserver.com");
    }

    #[test]
    fn multiple_aliases_one_block() {
        let c = "Host a b\n  HostName shared\n";
        let h = parse_ssh_config(c, "/h");
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].hostname, Some("shared".into()));
        assert_eq!(h[1].alias, "b");
        assert_eq!(h[1].hostname, Some("shared".into()));
    }

    #[test]
    fn case_insensitive_and_equals() {
        let c = "host srv\n  hostname=example.com\n  PORT 22\n";
        let h = parse_ssh_config(c, "/h");
        assert_eq!(h[0].hostname, Some("example.com".into()));
        assert_eq!(h[0].port, Some(22));
    }
}
