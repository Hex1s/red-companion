//! Local user storage on SQLite. Documents are opaque JSON strings here; their
//! schema lives in `rules-core`.

use std::path::Path;
use std::time::Duration;

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
    /// The file was last opened by a newer build, whose schema this one does not know.
    #[error("the database schema is version {found}, but this build only knows versions up to {supported}")]
    SchemaTooNew { found: i32, supported: i32 },
    /// The safety copy before a migration could not be made, so nothing was migrated.
    #[error("could not back up the database before migrating: {0}")]
    Backup(String),
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
    /// The session this character is an instance of; `None` for a library character.
    pub owner_session: Option<i64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Session,
    Folder,
}

/// A node of the explorer tree. `active` is only ever true for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRow {
    pub id: i64,
    pub kind: ContainerKind,
    pub parent_id: Option<i64>,
    pub name: String,
    pub active: bool,
}

const CONTAINER_SELECT: &str = "SELECT c.id, c.kind, c.parent_id, c.name, COALESCE(s.active, 0)      FROM containers c LEFT JOIN sessions s ON s.id = c.id";

/// A named JSON document (user template or encounter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRow {
    pub id: i64,
    pub name: String,
    pub data: String,
}

/// Versions recorded by refinery in this database; empty for a brand-new file.
fn applied_versions(conn: &Connection) -> Result<Vec<i32>> {
    let has_history: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history')",
        [],
        |r| r.get(0),
    )?;
    if !has_history {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare("SELECT version FROM refinery_schema_history")?;
    let rows = stmt.query_map([], |r| r.get::<_, i32>(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Refuses a database from a newer build, and copies one that is about to be migrated.
fn guard_schema(conn: &Connection, path: &Path) -> Result<()> {
    let known: Vec<i32> = embedded::migrations::runner().get_migrations().iter().map(|m| m.version()).collect();
    let supported = known.iter().copied().max().unwrap_or(0);
    let applied = applied_versions(conn)?;
    if let Some(found) = applied.iter().copied().filter(|v| !known.contains(v)).max() {
        return Err(StorageError::SchemaTooNew { found, supported });
    }
    let pending = known.iter().any(|v| !applied.contains(v));
    if pending && let Some(from) = applied.iter().copied().max() {
        back_up_before_migration(conn, path, from).map_err(|e| StorageError::Backup(e.to_string()))?;
    }
    Ok(())
}

/// `VACUUM INTO` makes a consistent copy even while the file is in WAL mode.
fn back_up_before_migration(conn: &Connection, path: &Path, from_version: i32) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dir = path.parent().unwrap_or_else(|| Path::new(".")).join(BACKUP_DIR);
    std::fs::create_dir_all(&dir)?;
    let stamp: String = conn.query_row("SELECT strftime('%Y%m%d-%H%M%S', 'now')", [], |r| r.get(0))?;
    let target = dir.join(format!("{BACKUP_PREFIX}{stamp}-v{from_version}.db"));
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;

    // The stamp leads the name, so name order is time order.
    let mut copies: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(BACKUP_PREFIX) && n.ends_with(".db")))
        .collect();
    copies.sort();
    let excess = copies.len().saturating_sub(BACKUPS_KEPT);
    for old in &copies[..excess] {
        let _ = std::fs::remove_file(old);
    }
    Ok(())
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

/// How many pre-migration copies are kept in `backups/` next to the database.
const BACKUPS_KEPT: usize = 5;
pub const BACKUP_DIR: &str = "backups";
const BACKUP_PREFIX: &str = "pre-migration-";

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Opens (creating if needed) the database file and applies migrations.
    ///
    /// A database that already holds data and needs migrating is copied to `backups/` first; one written
    /// by a newer build is refused untouched (`SchemaTooNew`), since migrations only go forward.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        guard_schema(&conn, path)?;
        // WAL keeps readers and the writer from blocking each other and survives a crash mid-write better.
        let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        Self::init(conn)
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

    /// Inserts a character as an instance owned by `session_id` (it is deleted with the session).
    pub fn insert_instance(&self, kind: CharacterKind, name: &str, data: &str, session_id: i64) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO characters (kind, name, data, owner_session_id) VALUES (?1, ?2, ?3, ?4)",
            params![kind.as_str(), name, data, session_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Hands a character to a session (`Some`) or to the library (`None`).
    pub fn set_character_owner(&self, id: i64, owner: Option<i64>) -> Result<bool> {
        Ok(self.conn.execute("UPDATE characters SET owner_session_id = ?2 WHERE id = ?1", params![id, owner])? > 0)
    }

    /// The instances of one session, oldest first.
    pub fn instances_of(&self, session_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM characters WHERE owner_session_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Every instance as `(session, character)`, for building the explorer tree in one pass.
    pub fn all_instances(&self) -> Result<Vec<(i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT owner_session_id, id FROM characters WHERE owner_session_id IS NOT NULL ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
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
            .query_row("SELECT id, name, data, kind, owner_session_id FROM characters WHERE id = ?1", [id], character_row)
            .optional()?)
    }

    pub fn list_characters(&self, kind: CharacterKind) -> Result<Vec<CharacterRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, data, kind, owner_session_id FROM characters WHERE kind = ?1 ORDER BY name, id")?;
        let rows = stmt.query_map([kind.as_str()], character_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_character(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM characters WHERE id = ?1", [id])? > 0)
    }

    /// Puts a deleted character back under its old id (ids are never reused, so it is free).
    /// False, and nothing changes, when a character with that id exists.
    /// `owner` is the session the character was an instance of; the caller passes it only while that
    /// session still exists.
    pub fn restore_character(
        &self,
        id: i64,
        kind: CharacterKind,
        name: &str,
        data: &str,
        owner: Option<i64>,
    ) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO characters (id, kind, name, data, owner_session_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, kind.as_str(), name, data, owner],
        )?;
        Ok(n > 0)
    }

    /// The containers holding a shortcut to a character.
    pub fn containers_of(&self, character_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT container_id FROM container_characters WHERE character_id = ?1 ORDER BY rowid",
        )?;
        let rows = stmt.query_map([character_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
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

    /// Creates a session: a root container plus its session row.
    pub fn create_session(&self, name: &str) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("INSERT INTO containers (kind, name) VALUES ('session', ?1)", [name])?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO sessions (id) VALUES (?1)", [id])?;
        tx.commit()?;
        Ok(id)
    }

    /// Creates a folder at the root, or inside `parent`. Folders nest one level only: `None` when
    /// `parent` does not exist or is not a root folder.
    pub fn create_folder(&self, parent: Option<i64>, name: &str) -> Result<Option<i64>> {
        if let Some(parent) = parent {
            let root_folder = self
                .container(parent)?
                .is_some_and(|c| c.kind == ContainerKind::Folder && c.parent_id.is_none());
            if !root_folder {
                return Ok(None);
            }
        }
        self.conn.execute(
            "INSERT INTO containers (kind, parent_id, name) VALUES ('folder', ?1, ?2)",
            params![parent, name],
        )?;
        Ok(Some(self.conn.last_insert_rowid()))
    }

    pub fn container(&self, id: i64) -> Result<Option<ContainerRow>> {
        Ok(self
            .conn
            .query_row(&format!("{CONTAINER_SELECT} WHERE c.id = ?1"), [id], container_row)
            .optional()?)
    }

    /// Every container, sessions and folders alike, in creation order.
    pub fn list_containers(&self) -> Result<Vec<ContainerRow>> {
        let mut stmt = self.conn.prepare(&format!("{CONTAINER_SELECT} ORDER BY c.id"))?;
        let rows = stmt.query_map([], container_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        Ok(self
            .list_containers()?
            .into_iter()
            .filter(|c| c.kind == ContainerKind::Session)
            .map(|c| SessionRow { id: c.id, name: c.name, active: c.active })
            .collect())
    }

    /// Moves a folder to the root (`parent` `None`) or into a root folder. False, and nothing
    /// changes, when that would break the rules: `id` is not a folder, the parent is not a root
    /// folder (or is the folder itself), or the folder has subfolders and would nest a second level.
    pub fn move_folder(&self, id: i64, parent: Option<i64>) -> Result<bool> {
        let Some(folder) = self.container(id)?.filter(|c| c.kind == ContainerKind::Folder) else {
            return Ok(false);
        };
        if let Some(parent) = parent {
            let root_folder = self
                .container(parent)?
                .is_some_and(|c| c.kind == ContainerKind::Folder && c.parent_id.is_none());
            let has_subfolders: bool = self.conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM containers WHERE parent_id = ?1)",
                [id],
                |r| r.get(0),
            )?;
            if parent == id || !root_folder || has_subfolders {
                return Ok(false);
            }
        }
        if folder.parent_id != parent {
            self.conn
                .execute("UPDATE containers SET parent_id = ?2 WHERE id = ?1", params![id, parent])?;
        }
        Ok(true)
    }

    /// Renames a session or a folder.
    pub fn rename_container(&self, id: i64, name: &str) -> Result<bool> {
        Ok(self.conn.execute("UPDATE containers SET name = ?2 WHERE id = ?1", params![id, name])? > 0)
    }

    /// Makes `id` the only active session; false (and no change) when it does not exist.
    pub fn set_active_session(&self, id: i64) -> Result<bool> {
        let tx = self.conn.unchecked_transaction()?;
        // The unique index allows one active row, so the old one is cleared first.
        tx.execute("UPDATE sessions SET active = 0 WHERE active = 1 AND id <> ?1", [id])?;
        let found = tx.execute("UPDATE sessions SET active = 1 WHERE id = ?1", [id])? > 0;
        if found {
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

    /// The characters lying in a container, in the order they were put there.
    pub fn container_members(&self, container_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT character_id FROM container_characters WHERE container_id = ?1 ORDER BY rowid",
        )?;
        let rows = stmt.query_map([container_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Every shortcut as `(container, character)`, for building the explorer tree in one pass.
    pub fn all_memberships(&self) -> Result<Vec<(i64, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT container_id, character_id FROM container_characters ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Puts a shortcut to a character into a container; adding an existing one changes nothing.
    pub fn add_member(&self, container_id: i64, character_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO container_characters (container_id, character_id) VALUES (?1, ?2)",
            params![container_id, character_id],
        )?;
        Ok(())
    }

    pub fn remove_member(&self, container_id: i64, character_id: i64) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM container_characters WHERE container_id = ?1 AND character_id = ?2",
            params![container_id, character_id],
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

    /// Deletes a container with everything under it: a session's scenes, a folder's subfolders and
    /// the shortcuts (the characters stay).
    pub fn delete_container(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM containers WHERE id = ?1", [id])? > 0)
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

fn container_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ContainerRow> {
    Ok(ContainerRow {
        id: r.get(0)?,
        kind: if r.get::<_, String>(1)? == "session" {
            ContainerKind::Session
        } else {
            ContainerKind::Folder
        },
        parent_id: r.get(2)?,
        name: r.get(3)?,
        active: r.get::<_, i64>(4)? != 0,
    })
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
        owner_session: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_in(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("test.db")
    }

    fn backups_in(dir: &tempfile::TempDir) -> Vec<std::path::PathBuf> {
        match std::fs::read_dir(dir.path().join(BACKUP_DIR)) {
            Ok(entries) => entries.map(|e| e.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// A file as an older build left it: only the first migration applied, one character in it.
    fn old_database(path: &Path) {
        let mut conn = Connection::open(path).unwrap();
        embedded::migrations::runner().set_target(refinery::Target::Version(1)).run(&mut conn).unwrap();
        conn.execute("INSERT INTO characters (kind, name, data) VALUES ('pc', 'V', '{}')", []).unwrap();
    }

    /// Migrations are checksummed by refinery over their text: a CRLF checkout (a Windows CI runner without
    /// `.gitattributes`) makes the installed app refuse every database made by an LF build.
    #[test]
    fn embedded_migrations_use_lf_line_endings() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(!text.contains('\r'), "{} has CRLF line endings, see .gitattributes", path.display());
        }
    }

    #[test]
    fn a_new_file_is_created_without_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        Storage::open(&db_in(&dir)).unwrap();
        assert!(backups_in(&dir).is_empty());
    }

    #[test]
    fn an_old_schema_is_copied_before_it_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        old_database(&db_in(&dir));

        let storage = Storage::open(&db_in(&dir)).unwrap();
        assert_eq!(storage.list_characters(CharacterKind::Pc).unwrap().len(), 1);

        let copies = backups_in(&dir);
        assert_eq!(copies.len(), 1);
        let name = copies[0].file_name().unwrap().to_str().unwrap();
        assert!(name.ends_with("-v1.db"), "{name}");
        // The copy is the old schema with the old data, untouched by the migration.
        let copy = Connection::open(&copies[0]).unwrap();
        assert_eq!(applied_versions(&copy).unwrap(), vec![1]);
        let count: i64 = copy.query_row("SELECT COUNT(*) FROM characters", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn an_up_to_date_file_is_not_copied_again() {
        let dir = tempfile::tempdir().unwrap();
        old_database(&db_in(&dir));
        Storage::open(&db_in(&dir)).unwrap();
        Storage::open(&db_in(&dir)).unwrap();
        assert_eq!(backups_in(&dir).len(), 1);
    }

    #[test]
    fn only_the_newest_copies_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join(BACKUP_DIR);
        std::fs::create_dir_all(&backups).unwrap();
        for n in 0..BACKUPS_KEPT + 3 {
            std::fs::write(backups.join(format!("{BACKUP_PREFIX}20200101-00000{n}-v1.db")), "").unwrap();
        }
        old_database(&db_in(&dir));
        Storage::open(&db_in(&dir)).unwrap();

        let names: Vec<_> = backups_in(&dir).iter().map(|p| p.file_name().unwrap().to_str().unwrap().to_string()).collect();
        assert_eq!(names.len(), BACKUPS_KEPT);
        // The fresh copy (a current date) survives; the oldest placeholders are gone.
        assert!(names.iter().any(|n| !n.contains("2020")));
        assert!(!names.iter().any(|n| n.contains("20200101-000000")));
    }

    #[test]
    fn a_schema_from_a_newer_build_is_refused_and_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        Storage::open(&db_in(&dir)).unwrap();
        {
            let conn = Connection::open(db_in(&dir)).unwrap();
            conn.execute(
                "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) VALUES (999, 'future', '2099-01-01T00:00:00Z', '0')",
                [],
            )
            .unwrap();
        }
        match Storage::open(&db_in(&dir)) {
            Err(StorageError::SchemaTooNew { found: 999, .. }) => {}
            other => panic!("expected SchemaTooNew, got {:?}", other.map(|_| ())),
        }
        assert!(backups_in(&dir).is_empty());
    }

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
    fn instances_belong_to_a_session_and_go_with_it() {
        let s = Storage::open_in_memory().unwrap();
        let session = s.create_session("Night").unwrap();
        let other = s.create_session("Day").unwrap();
        let library = s.insert_character(CharacterKind::Npc, "Fixer", "{}").unwrap();
        let a = s.insert_instance(CharacterKind::Npc, "Cyberpsycho 1", "{}", session).unwrap();
        let b = s.insert_instance(CharacterKind::Npc, "Cyberpsycho 2", "{}", session).unwrap();
        let elsewhere = s.insert_instance(CharacterKind::Npc, "Ganger", "{}", other).unwrap();

        assert_eq!(s.get_character(library).unwrap().unwrap().owner_session, None);
        assert_eq!(s.get_character(a).unwrap().unwrap().owner_session, Some(session));
        assert_eq!(s.instances_of(session).unwrap(), vec![a, b]);
        assert_eq!(s.all_instances().unwrap(), vec![(session, a), (session, b), (other, elsewhere)]);

        // Saving to the library clears the owner, so the character survives the session.
        assert!(s.set_character_owner(b, None).unwrap());
        s.delete_container(session).unwrap();
        assert!(s.get_character(a).unwrap().is_none());
        assert!(s.get_character(b).unwrap().is_some());
        assert!(s.get_character(library).unwrap().is_some());
        assert!(s.get_character(elsewhere).unwrap().is_some());
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

        s.delete_container(session).unwrap();
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

        assert!(s.rename_container(two, "Renamed").unwrap());
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
        assert_eq!(s.container_members(one).unwrap(), vec![v, gang]);
        assert_eq!(s.container_members(two).unwrap(), vec![v]);

        // Leaving a session does not delete the character.
        assert!(s.remove_member(one, gang).unwrap());
        assert!(!s.remove_member(one, gang).unwrap());
        assert!(s.get_character(gang).unwrap().is_some());

        // Deleting a character removes it from every session.
        s.delete_character(v).unwrap();
        assert!(s.container_members(one).unwrap().is_empty());
        assert!(s.container_members(two).unwrap().is_empty());

        // Deleting a session keeps its characters.
        s.add_member(two, gang).unwrap();
        s.delete_container(two).unwrap();
        assert!(s.get_character(gang).unwrap().is_some());
        assert!(s.container_members(two).unwrap().is_empty());
    }

    #[test]
    fn folders_nest_one_level_and_hold_shortcuts() {
        let s = Storage::open_in_memory().unwrap();
        let gangs = s.create_folder(None, "Gangs").unwrap().unwrap();
        let maelstrom = s.create_folder(Some(gangs), "Maelstrom").unwrap().unwrap();
        // A third level, a session as parent and an unknown parent are all refused.
        assert!(s.create_folder(Some(maelstrom), "Deeper").unwrap().is_none());
        let session = s.create_session("Night 1").unwrap();
        assert!(s.create_folder(Some(session), "In session").unwrap().is_none());
        assert!(s.create_folder(Some(999), "Nowhere").unwrap().is_none());

        let tree = s.list_containers().unwrap();
        assert_eq!(tree.len(), 3);
        let child = tree.iter().find(|c| c.id == maelstrom).unwrap();
        assert_eq!((child.kind, child.parent_id), (ContainerKind::Folder, Some(gangs)));
        // Only sessions are listed as sessions.
        assert_eq!(s.list_sessions().unwrap().len(), 1);

        // One character may lie in a folder and a session at once.
        let ganger = s.insert_character(CharacterKind::Npc, "Ganger", "{}").unwrap();
        s.add_member(maelstrom, ganger).unwrap();
        s.add_member(session, ganger).unwrap();
        assert_eq!(s.all_memberships().unwrap(), vec![(maelstrom, ganger), (session, ganger)]);

        // Deleting the parent folder takes the subfolder and its shortcuts; the character stays.
        assert!(s.delete_container(gangs).unwrap());
        assert!(s.container(maelstrom).unwrap().is_none());
        assert_eq!(s.all_memberships().unwrap(), vec![(session, ganger)]);
        assert!(s.get_character(ganger).unwrap().is_some());
    }

    #[test]
    fn folders_move_between_the_root_and_root_folders_within_the_depth_limit() {
        let s = Storage::open_in_memory().unwrap();
        let gangs = s.create_folder(None, "Gangs").unwrap().unwrap();
        let places = s.create_folder(None, "Places").unwrap().unwrap();
        let maelstrom = s.create_folder(Some(gangs), "Maelstrom").unwrap().unwrap();
        let session = s.create_session("Night 1").unwrap();

        // Into another root folder, and back to the root.
        assert!(s.move_folder(maelstrom, Some(places)).unwrap());
        assert_eq!(s.container(maelstrom).unwrap().unwrap().parent_id, Some(places));
        assert!(s.move_folder(maelstrom, None).unwrap());
        assert_eq!(s.container(maelstrom).unwrap().unwrap().parent_id, None);

        // A folder with a subfolder cannot go inside another one (a third level), nor into itself.
        assert!(s.move_folder(maelstrom, Some(gangs)).unwrap());
        assert!(!s.move_folder(gangs, Some(places)).unwrap());
        assert!(!s.move_folder(places, Some(places)).unwrap());
        // Not into a nested folder, a session, or nowhere; sessions do not move at all.
        let loose = s.create_folder(None, "Loose").unwrap().unwrap();
        assert!(!s.move_folder(loose, Some(maelstrom)).unwrap());
        assert!(!s.move_folder(loose, Some(session)).unwrap());
        assert!(!s.move_folder(loose, Some(999)).unwrap());
        assert!(!s.move_folder(session, None).unwrap());
        assert_eq!(s.container(gangs).unwrap().unwrap().parent_id, None);
    }

    #[test]
    fn a_folder_is_never_active_and_holds_no_scene() {
        let s = Storage::open_in_memory().unwrap();
        let folder = s.create_folder(None, "Gangs").unwrap().unwrap();
        assert!(!s.set_active_session(folder).unwrap());
        assert!(s.active_session().unwrap().is_none());
        assert!(s.create_scene(folder, "x", "{}").is_err());
        assert!(!s.container(folder).unwrap().unwrap().active);
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
