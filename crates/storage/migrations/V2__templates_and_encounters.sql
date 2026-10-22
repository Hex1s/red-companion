-- The GM's own NPC templates: `data` is a fresh character document (schema owned by rules-core).
CREATE TABLE user_templates (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    data       TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Encounters: `data` is the encounter document (name, notes, template lines).
CREATE TABLE encounters (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    data       TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
