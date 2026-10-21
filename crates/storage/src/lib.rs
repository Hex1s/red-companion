//! Local user storage on SQLite. Documents are opaque JSON strings here; their
//! schema lives in `rules-core`.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

mod embedded {
    refinery::embed_migrations!("migrations");
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterKind {
    Pc,
    Npc,
}

impl CharacterKind {
    fn parse(text: &str) -> Self {
        if text == "npc" { Self::Npc } else { Self::Pc }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pc => "pc",
            Self::Npc => "npc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRow {
    pub id: i64,
    pub name: String,
    pub data: String,
    pub kind: CharacterKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneRow {
    pub id: i64,
    pub session_id: i64,
    pub name: String,
    pub data: String,
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Opens (creating if needed) the database file and applies migrations.
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        embedded::migrations::runner().run(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn insert_character(&self, kind: CharacterKind, name: &str, data: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO characters (kind, name, data) VALUES (?1, ?2, ?3)",
            params![kind.as_str(), name, data],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_character(&self, id: i64, name: &str, data: &str) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE characters SET name = ?2, data = ?3, updated_at = datetime('now') WHERE id = ?1",
            params![id, name, data],
        )?;
        Ok(n > 0)
    }

    pub fn get_character(&self, id: i64) -> Result<Option<CharacterRow>> {
        Ok(self
            .conn
            .query_row("SELECT id, name, data, kind FROM characters WHERE id = ?1", [id], character_row)
            .optional()?)
    }

    pub fn list_characters(&self, kind: CharacterKind) -> Result<Vec<CharacterRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, data, kind FROM characters WHERE kind = ?1 ORDER BY name, id")?;
        let rows = stmt.query_map([kind.as_str()], character_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_character(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM characters WHERE id = ?1", [id])? > 0)
    }

    pub fn create_session(&self, name: &str) -> Result<i64> {
        self.conn.execute("INSERT INTO sessions (name) VALUES (?1)", [name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_session_notes(&self, id: i64, notes: &str) -> Result<bool> {
        Ok(self.conn.execute("UPDATE sessions SET notes = ?2 WHERE id = ?1", params![id, notes])? > 0)
    }

    pub fn session_notes(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT notes FROM sessions WHERE id = ?1", [id], |r| r.get(0))
            .optional()?)
    }

    /// Deleting a session deletes its scenes (ON DELETE CASCADE).
    pub fn delete_session(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM sessions WHERE id = ?1", [id])? > 0)
    }

    pub fn create_scene(&self, session_id: i64, name: &str, data: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO scenes (session_id, name, data) VALUES (?1, ?2, ?3)",
            params![session_id, name, data],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn save_scene(&self, id: i64, data: &str) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE scenes SET data = ?2, updated_at = datetime('now') WHERE id = ?1",
            params![id, data],
        )?;
        Ok(n > 0)
    }

    pub fn list_scenes(&self, session_id: i64) -> Result<Vec<SceneRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, name, data FROM scenes WHERE session_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([session_id], |r| {
            Ok(SceneRow { id: r.get(0)?, session_id: r.get(1)?, name: r.get(2)?, data: r.get(3)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// The most recently created scene: the one the app works with.
    pub fn latest_scene(&self) -> Result<Option<SceneRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, session_id, name, data FROM scenes ORDER BY id DESC LIMIT 1",
                [],
                |r| {
                    Ok(SceneRow {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        name: r.get(2)?,
                        data: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }
}

fn character_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<CharacterRow> {
    Ok(CharacterRow {
        id: r.get(0)?,
        name: r.get(1)?,
        data: r.get(2)?,
        kind: CharacterKind::parse(&r.get::<_, String>(3)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_crud_and_kinds() {
        let s = Storage::open_in_memory().unwrap();
        let pc = s.insert_character(CharacterKind::Pc, "V", r#"{"body":6}"#).unwrap();
        s.insert_character(CharacterKind::Npc, "Boosterganger", "{}").unwrap();

        assert_eq!(s.list_characters(CharacterKind::Pc).unwrap().len(), 1);
        assert_eq!(s.list_characters(CharacterKind::Npc).unwrap().len(), 1);
        assert_eq!(s.get_character(pc).unwrap().unwrap().data, r#"{"body":6}"#);

        assert!(s.update_character(pc, "Vee", r#"{"body":7}"#).unwrap());
        assert_eq!(s.get_character(pc).unwrap().unwrap().name, "Vee");

        assert!(s.delete_character(pc).unwrap());
        assert!(s.get_character(pc).unwrap().is_none());
        assert!(!s.delete_character(pc).unwrap());
    }

    #[test]
    fn scenes_cascade_with_session() {
        let s = Storage::open_in_memory().unwrap();
        let session = s.create_session("Night 1").unwrap();
        let scene = s.create_scene(session, "Ambush", "{}").unwrap();
        assert!(s.save_scene(scene, r#"{"cover":[{"hp":10}]}"#).unwrap());
        assert_eq!(s.list_scenes(session).unwrap()[0].data, r#"{"cover":[{"hp":10}]}"#);

        s.set_session_notes(session, "notes").unwrap();
        assert_eq!(s.session_notes(session).unwrap().as_deref(), Some("notes"));

        s.delete_session(session).unwrap();
        assert!(s.list_scenes(session).unwrap().is_empty());
    }

    #[test]
    fn latest_scene_is_the_newest() {
        let s = Storage::open_in_memory().unwrap();
        assert!(s.latest_scene().unwrap().is_none());
        let session = s.create_session("Night 1").unwrap();
        s.create_scene(session, "first", "{}").unwrap();
        let second = s.create_scene(session, "second", "{}").unwrap();
        assert_eq!(s.latest_scene().unwrap().unwrap().id, second);
    }

    #[test]
    fn scene_requires_existing_session() {
        let s = Storage::open_in_memory().unwrap();
        assert!(s.create_scene(999, "x", "{}").is_err());
    }

    #[test]
    fn reopening_file_keeps_data_and_migrations_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("rc-storage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);

        let id = Storage::open(&path).unwrap().insert_character(CharacterKind::Pc, "V", "{}").unwrap();
        let again = Storage::open(&path).unwrap();
        assert_eq!(again.get_character(id).unwrap().unwrap().name, "V");

        drop(again);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
