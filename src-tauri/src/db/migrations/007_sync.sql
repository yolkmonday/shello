-- 007_sync.sql
-- Add sync tracking columns to all syncable tables

ALTER TABLE profiles ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'synced';
ALTER TABLE profiles ADD COLUMN remote_updated_at TEXT;

ALTER TABLE groups ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'synced';
ALTER TABLE groups ADD COLUMN remote_updated_at TEXT;

ALTER TABLE snippets ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'synced';
ALTER TABLE snippets ADD COLUMN remote_updated_at TEXT;

ALTER TABLE custom_recipes ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'synced';
ALTER TABLE custom_recipes ADD COLUMN remote_updated_at TEXT;

-- Device ID for sync metadata tracking
CREATE TABLE IF NOT EXISTS sync_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
