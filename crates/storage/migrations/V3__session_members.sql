-- Sessions become the unit of play: each has its own scene (cover, fight) and its own members.
-- Old sessions and scenes are dropped (decision of the project owner: the app is under active
-- development); characters, templates and encounters are kept.
DELETE FROM sessions;

-- Exactly one session is active: the one the app works with. Kept in sync by the storage code.
ALTER TABLE sessions ADD COLUMN active INTEGER NOT NULL DEFAULT 0;

-- Who takes part in a session. Removing a member is not deleting the character.
CREATE TABLE session_characters (
    session_id   INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, character_id)
);

CREATE INDEX idx_session_characters_character ON session_characters(character_id);
