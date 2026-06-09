use serde::Serialize;

/// A normalized file-system entry, used for both local and remote panes.
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    /// Unix seconds since epoch, if known.
    pub modified: Option<i64>,
    /// Unix permission bits (mode), if known.
    pub mode: Option<u32>,
}

/// Join a directory path and an entry name using forward slashes (remote/POSIX).
pub fn join_remote(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}
