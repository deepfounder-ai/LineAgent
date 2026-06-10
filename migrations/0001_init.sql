-- lineagent initial schema
-- Multi-tenant per-user issue tracker for AI agents.
--
-- NOTE: PRAGMAs that cannot be changed inside a transaction (journal_mode,
-- synchronous, foreign_keys) are applied in `src/storage/pool.rs` *before*
-- the migration transaction is opened. Keeping them out of this file avoids
-- the SQLite error: "Safety level may not be changed inside a transaction".

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY NOT NULL,
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ---------------------------------------------------------------------------
-- api_keys
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_keys (
    id              TEXT PRIMARY KEY NOT NULL,
    user_id         TEXT NOT NULL,
    name            TEXT NOT NULL,
    key_hash        TEXT NOT NULL UNIQUE,
    created_at      TEXT NOT NULL,
    last_used_at    TEXT,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);

-- ---------------------------------------------------------------------------
-- events
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    ref         TEXT,
    ts          TEXT    NOT NULL,
    payload_json TEXT,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);
