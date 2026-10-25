-- The explorer tree: sessions and folders are containers that hold shortcuts to characters.
-- Old sessions, scenes and memberships are dropped (decision of the project owner: the app is under
-- active development); characters, templates and encounters are kept.
DROP TABLE scenes;
DROP TABLE session_characters;
DROP TABLE sessions;

-- A folder may sit in a root folder (one level of nesting, checked by the storage code); a session
-- is always at the root.
CREATE TABLE containers (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       TEXT NOT NULL CHECK (kind IN ('session', 'folder')),
    parent_id  INTEGER REFERENCES containers(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (kind = 'folder' OR parent_id IS NULL)
);

CREATE INDEX idx_containers_parent ON containers(parent_id);

-- What only a session has: notes and the "working session" flag. Scenes reference this table, so a
-- folder can neither hold a scene nor be active, and the schema says so.
CREATE TABLE sessions (
    id     INTEGER PRIMARY KEY REFERENCES containers(id) ON DELETE CASCADE,
    notes  TEXT NOT NULL DEFAULT '',
    active INTEGER NOT NULL DEFAULT 0
);

-- At most one session is active.
CREATE UNIQUE INDEX idx_sessions_one_active ON sessions(active) WHERE active = 1;

-- Shortcuts: a character may lie in many containers. Removing one is not deleting the character.
CREATE TABLE container_characters (
    container_id INTEGER NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    PRIMARY KEY (container_id, character_id)
);

CREATE INDEX idx_container_characters_character ON container_characters(character_id);

-- A scene holds cover objects, initiative state and the event history.
CREATE TABLE scenes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    data       TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_scenes_session ON scenes(session_id);
