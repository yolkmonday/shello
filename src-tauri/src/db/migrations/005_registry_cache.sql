CREATE TABLE IF NOT EXISTS registry_cache (
    key         TEXT PRIMARY KEY,
    version     TEXT NOT NULL DEFAULT '',
    data        TEXT NOT NULL,
    fetched_at  TEXT NOT NULL
);
