-- Vault configuration (singleton row)
CREATE TABLE IF NOT EXISTS vault_config (
    id          INTEGER PRIMARY KEY CHECK(id = 1),
    salt        BLOB NOT NULL,
    verify_blob BLOB NOT NULL,
    created_at  TEXT NOT NULL
);

-- Recreate profiles table with encrypted credential columns (BLOB instead of TEXT).
-- SQLite doesn't support renaming columns in all versions, so we recreate.

CREATE TABLE profiles_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    host        TEXT NOT NULL,
    port        INTEGER NOT NULL DEFAULT 22,
    username    TEXT NOT NULL,
    auth_type   TEXT NOT NULL CHECK(auth_type IN ('password', 'key')),
    password_enc   BLOB,
    key_path_enc   BLOB,
    passphrase_enc BLOB,
    group_id    TEXT REFERENCES groups(id) ON DELETE SET NULL,
    tags        TEXT NOT NULL DEFAULT '',
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

INSERT INTO profiles_new (id, name, host, port, username, auth_type,
    password_enc, key_path_enc, passphrase_enc,
    group_id, tags, sort_order, created_at, updated_at)
SELECT id, name, host, port, username, auth_type,
    CAST(password AS BLOB), CAST(key_path AS BLOB), CAST(passphrase AS BLOB),
    group_id, tags, sort_order, created_at, updated_at
FROM profiles;

DROP TABLE profiles;
ALTER TABLE profiles_new RENAME TO profiles;
