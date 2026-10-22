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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: i64,
    pub name: String,
    pub active: bool,
}

/// A named JSON document (user template or encounter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRow {
    pub id: i64,
    pub name: String,
    pub data: String,
}

/// The tables that hold `DocRow`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTable {
    UserTemplates,
    Encounters,
}

impl DocTable {
    fn name(self) -> &'static str {
        match self {
            Self::UserTemplates => "user_templates",
            Self::Encounters => "encounters",
        }
    }
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

    pub fn insert_doc(&self, table: DocTable, name: &str, data: &str) -> Result<i64> {
        self.conn.execute(
            &format!("INSERT INTO {} (name, data) VALUES (?1, ?2)", table.name()),
            params![name, data],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_doc(&self, table: DocTable, id: i64, name: &str, data: &str) -> Result<bool> {
        let n = self.conn.execute(
            &format!(
                "UPDATE {} SET name = ?2, data = ?3, updated_at = datetime('now') WHERE id = ?1",
                table.name()
            ),
            params![id, name, data],
        )?;
        Ok(n > 0)
    }

    pub fn get_doc(&self, table: DocTable, id: i64) -> Result<Option<DocRow>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT id, name, data FROM {} WHERE id = ?1", table.name()),
                [id],
                doc_row,
            )
            .optional()?)
    }

    pub fn list_docs(&self, table: DocTable) -> Result<Vec<DocRow>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT id, name, data FROM {} ORDER BY name, id", table.name()))?;
        let rows = stmt.query_map([], doc_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_doc(&self, table: DocTable, id: i64) -> Result<bool> {
        Ok(self
            .conn
            .execute(&format!("DELETE FROM {} WHERE id = ?1", table.name()), [id])?
            > 0)
    }

    pub fn create_session(&self, name: &str) -> Result<i64> {
        self.conn.execute("INSERT INTO sessions (name) VALUES (?1)", [name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare("SELECT id, name, active FROM sessions ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionRow { id: r.get(0)?, name: r.get(1)?, active: r.get::<_, i64>(2)? != 0 })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn rename_session(&self, id: i64, name: &str) -> Result<bool> {
        Ok(self.conn.execute("UPDATE sessions SET name = ?2 WHERE id = ?1", params![id, name])? > 0)
    }

    /// Makes `id` the only active session; false (and no change) when it does not exist.
    pub fn set_active_session(&self, id: i64) -> Result<bool> {
        let tx = self.conn.unchecked_transaction()?;
        let found = tx.execute("UPDATE sessions SET active = 1 WHERE id = ?1", [id])? > 0;
        if found {
            tx.execute("UPDATE sessions SET active = 0 WHERE id <> ?1", [id])?;
            tx.commit()?;
        }
        Ok(found)
    }

    pub fn active_session(&self) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT id FROM sessions WHERE active = 1 LIMIT 1", [], |r| r.get(0))
            .optional()?)
    }

    /// The characters taking part in a session, in the order they joined.
    pub fn session_members(&self, session_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT character_id FROM session_characters WHERE session_id = ?1 ORDER BY rowid",
        )?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Adds a character to a session; adding an existing member changes nothing.
    pub fn add_member(&self, session_id: i64, character_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO session_characters (session_id, character_id) VALUES (?1, ?2)",
            params![session_id, character_id],
        )?;
        Ok(())
    }

    pub fn remove_member(&self, session_id: i64, character_id: i64) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM session_characters WHERE session_id = ?1 AND character_id = ?2",
            params![session_id, character_id],
        )? > 0)
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

    /// Every scene of every session (used to clean up after a deleted character).
    pub fn all_scenes(&self) -> Result<Vec<SceneRow>> {
        let mut stmt =
            self.conn.prepare("SELECT id, session_id, name, data FROM scenes ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(SceneRow { id: r.get(0)?, session_id: r.get(1)?, name: r.get(2)?, data: r.get(3)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// The first scene of the active session: the one the app works with.
    pub fn active_scene(&self) -> Result<Option<SceneRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT s.id, s.session_id, s.name, s.data FROM scenes s \
                 JOIN sessions n ON n.id = s.session_id WHERE n.active = 1 ORDER BY s.id LIMIT 1",
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

fn doc_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocRow> {
    Ok(DocRow { id: r.get(0)?, name: r.get(1)?, data: r.get(2)? })
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
    fn docs_live_in_separate_tables() {
        let s = Storage::open_in_memory().unwrap();
        let template = s.insert_doc(DocTable::UserTemplates, "Thug", r#"{"hp":20}"#).unwrap();
        let encounter = s.insert_doc(DocTable::Encounters, "Ambush", "{}").unwrap();

        assert_eq!(s.list_docs(DocTable::UserTemplates).unwrap().len(), 1);
        assert_eq!(s.list_docs(DocTable::Encounters).unwrap().len(), 1);
        assert_eq!(s.get_doc(DocTable::UserTemplates, template).unwrap().unwrap().name, "Thug");

        assert!(s.update_doc(DocTable::Encounters, encounter, "Big ambush", "{\"a\":1}").unwrap());
        let row = s.get_doc(DocTable::Encounters, encounter).unwrap().unwrap();
        assert_eq!((row.name.as_str(), row.data.as_str()), ("Big ambush", "{\"a\":1}"));

        assert!(s.delete_doc(DocTable::Encounters, encounter).unwrap());
        assert!(!s.delete_doc(DocTable::Encounters, encounter).unwrap());
        assert!(s.get_doc(DocTable::Encounters, encounter).unwrap().is_none());
        assert_eq!(s.list_docs(DocTable::UserTemplates).unwrap().len(), 1);
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
    fn active_session_owns_the_working_scene() {
        let s = Storage::open_in_memory().unwrap();
        assert!(s.active_scene().unwrap().is_none());
        assert!(s.active_session().unwrap().is_none());

        let one = s.create_session("One").unwrap();
        let two = s.create_session("Two").unwrap();
        let scene_one = s.create_scene(one, "Scene", "{}").unwrap();
        let scene_two = s.create_scene(two, "Scene", "{}").unwrap();
        // Nothing is active until one is chosen.
        assert!(s.active_scene().unwrap().is_none());

        assert!(s.set_active_session(one).unwrap());
        assert_eq!(s.active_scene().unwrap().unwrap().id, scene_one);
        assert!(s.set_active_session(two).unwrap());
        assert_eq!(s.active_scene().unwrap().unwrap().id, scene_two);
        assert_eq!(s.active_session().unwrap(), Some(two));
        let sessions = s.list_sessions().unwrap();
        assert_eq!(sessions.iter().filter(|x| x.active).count(), 1);

        // An unknown id changes nothing.
        assert!(!s.set_active_session(999).unwrap());
        assert_eq!(s.active_session().unwrap(), Some(two));

        assert!(s.rename_session(two, "Renamed").unwrap());
        assert_eq!(s.list_sessions().unwrap()[1].name, "Renamed");
    }

    #[test]
    fn members_are_per_session_and_cascade() {
        let s = Storage::open_in_memory().unwrap();
        let one = s.create_session("One").unwrap();
        let two = s.create_session("Two").unwrap();
        let v = s.insert_character(CharacterKind::Pc, "V", "{}").unwrap();
        let gang = s.insert_character(CharacterKind::Npc, "Gang", "{}").unwrap();

        s.add_member(one, v).unwrap();
        s.add_member(one, gang).unwrap();
        s.add_member(one, v).unwrap(); // joining twice changes nothing
        s.add_member(two, v).unwrap();
        assert_eq!(s.session_members(one).unwrap(), vec![v, gang]);
        assert_eq!(s.session_members(two).unwrap(), vec![v]);

        // Leaving a session does not delete the character.
        assert!(s.remove_member(one, gang).unwrap());
        assert!(!s.remove_member(one, gang).unwrap());
        assert!(s.get_character(gang).unwrap().is_some());

        // Deleting a character removes it from every session.
        s.delete_character(v).unwrap();
        assert!(s.session_members(one).unwrap().is_empty());
        assert!(s.session_members(two).unwrap().is_empty());

        // Deleting a session keeps its characters.
        s.add_member(two, gang).unwrap();
        s.delete_session(two).unwrap();
        assert!(s.get_character(gang).unwrap().is_some());
        assert!(s.session_members(two).unwrap().is_empty());
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
