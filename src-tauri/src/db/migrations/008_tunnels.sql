CREATE TABLE IF NOT EXISTS tunnels (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    tunnel_type TEXT NOT NULL DEFAULT 'local',
    local_host TEXT NOT NULL DEFAULT '127.0.0.1',
    local_port INTEGER NOT NULL,
    remote_host TEXT NOT NULL,
    remote_port INTEGER NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tunnels_profile ON tunnels (profile_id);
