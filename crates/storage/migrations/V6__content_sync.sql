-- Content sync (RP-7): settings and the installation token, the downloaded reference data and which
-- packs are switched on. Nothing here touches the user's own data (characters, sessions, scenes).

-- One row per key: the installation id and token, the mode (online / local), the chosen languages,
-- the last accepted manifest. The token is per installation and can be revoked on the server.
CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

-- One collection of one pack in one language, exactly as the server sent it. The raw JSON text is
-- kept because the hash in the signed manifest is taken over it, not over a typed copy.
CREATE TABLE content_cache (
    pack           TEXT NOT NULL,
    collection     TEXT NOT NULL,
    lang           TEXT NOT NULL,
    hash           TEXT NOT NULL,
    release_number INTEGER NOT NULL,
    body           TEXT NOT NULL,
    fetched_at     TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (pack, collection, lang)
) WITHOUT ROWID;

-- A pack can be switched off without deleting what was downloaded; "downloaded" is simply rows in
-- content_cache. A pack with no row here is on.
CREATE TABLE content_packs (
    code    TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1))
) WITHOUT ROWID;
