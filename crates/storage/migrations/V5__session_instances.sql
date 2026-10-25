-- Instances: an NPC copied from a template for one session. It belongs to that session and goes
-- away with it; "save to the library" clears the owner and makes it an ordinary character. Player
-- characters are never instances (checked by the app, not by the schema).
ALTER TABLE characters ADD COLUMN owner_session_id INTEGER REFERENCES sessions(id) ON DELETE CASCADE;

CREATE INDEX idx_characters_owner_session ON characters(owner_session_id);
