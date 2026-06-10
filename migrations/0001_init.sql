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

-- ---------------------------------------------------------------------------
-- projects
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS projects (
    id              TEXT    PRIMARY KEY NOT NULL,
    user_id         TEXT    NOT NULL,
    key             TEXT    NOT NULL,
    name            TEXT    NOT NULL,
    description     TEXT,
    ticket_counter  INTEGER NOT NULL DEFAULT 0,
    cycle_counter   INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL,
    UNIQUE(user_id, key),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ---------------------------------------------------------------------------
-- cycles (declared before tickets because tickets.cycle_id references cycles)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS cycles (
    id          TEXT    PRIMARY KEY NOT NULL,
    user_id     TEXT    NOT NULL,
    project_id  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    starts_at   TEXT,
    ends_at     TEXT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    FOREIGN KEY(user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- ---------------------------------------------------------------------------
-- tickets
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tickets (
    id          TEXT    PRIMARY KEY NOT NULL,
    user_id     TEXT    NOT NULL,
    project_id  TEXT    NOT NULL,
    number      INTEGER NOT NULL,
    identifier  TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    description TEXT,
    status      TEXT    NOT NULL DEFAULT 'backlog',
    priority    TEXT    NOT NULL DEFAULT 'medium',
    assignee    TEXT,
    parent_id   TEXT,
    cycle_id    TEXT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    UNIQUE(user_id, identifier),
    UNIQUE(project_id, number),
    FOREIGN KEY(user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_id)  REFERENCES tickets(id)  ON DELETE SET NULL,
    FOREIGN KEY(cycle_id)   REFERENCES cycles(id)   ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_tickets_user_status   ON tickets(user_id, status);
CREATE INDEX IF NOT EXISTS idx_tickets_user_project  ON tickets(user_id, project_id);
CREATE INDEX IF NOT EXISTS idx_tickets_user_assignee ON tickets(user_id, assignee);
CREATE INDEX IF NOT EXISTS idx_tickets_user_parent   ON tickets(user_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_tickets_user_cycle    ON tickets(user_id, cycle_id);

-- ---------------------------------------------------------------------------
-- comments
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS comments (
    id          TEXT PRIMARY KEY NOT NULL,
    user_id     TEXT NOT NULL,
    ticket_id   TEXT NOT NULL,
    author      TEXT,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    FOREIGN KEY(user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    FOREIGN KEY(ticket_id) REFERENCES tickets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_comments_ticket ON comments(ticket_id);

-- ---------------------------------------------------------------------------
-- relations
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS relations (
    id              TEXT PRIMARY KEY NOT NULL,
    user_id         TEXT NOT NULL,
    from_ticket_id  TEXT NOT NULL,
    to_ticket_id    TEXT NOT NULL,
    type            TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(from_ticket_id, to_ticket_id, type),
    FOREIGN KEY(user_id)         REFERENCES users(id)   ON DELETE CASCADE,
    FOREIGN KEY(from_ticket_id)  REFERENCES tickets(id) ON DELETE CASCADE,
    FOREIGN KEY(to_ticket_id)    REFERENCES tickets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(user_id, from_ticket_id);
CREATE INDEX IF NOT EXISTS idx_relations_to   ON relations(user_id, to_ticket_id);

-- ---------------------------------------------------------------------------
-- tickets_fts  (FTS5 virtual table + sync triggers)
-- ---------------------------------------------------------------------------
CREATE VIRTUAL TABLE IF NOT EXISTS tickets_fts USING fts5(
    title,
    description,
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS tickets_ai AFTER INSERT ON tickets BEGIN
    INSERT INTO tickets_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description, ''));
END;

CREATE TRIGGER IF NOT EXISTS tickets_ad AFTER DELETE ON tickets BEGIN
    DELETE FROM tickets_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS tickets_au AFTER UPDATE ON tickets BEGIN
    DELETE FROM tickets_fts WHERE rowid = old.rowid;
    INSERT INTO tickets_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description, ''));
END;
