//! Storage of the content sync (RP-7): settings and the installation token, the downloaded
//! reference data and the on/off switch of packs. The data is opaque text here, like everywhere in
//! this crate: the raw JSON the server sent is kept and checked against the signed manifest by
//! `rules-core::manifest`.

use rusqlite::{OptionalExtension, params};

use crate::{Result, Storage};

/// One downloaded collection: a pack's collection in one language, as the server sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedCollection {
    pub pack: String,
    pub collection: String,
    pub lang: String,
    /// The hash from the signed manifest it was accepted under.
    pub hash: String,
    /// The content release it belongs to.
    pub release: i64,
    /// The server's JSON, untouched.
    pub body: String,
}

/// What is cached, without the bodies: enough to compare with a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub pack: String,
    pub collection: String,
    pub lang: String,
    pub hash: String,
    pub release: i64,
}

impl Storage {
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// `false`: there was no such setting.
    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM app_settings WHERE key = ?1", [key])? > 0)
    }

    /// Stores collections, replacing the ones of the same pack, collection and language. All or
    /// nothing: a sync step that stops half-way leaves the previous, consistent set.
    pub fn put_cached(&self, collections: &[CachedCollection]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for c in collections {
            tx.execute(
                "INSERT INTO content_cache (pack, collection, lang, hash, release_number, body, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                 ON CONFLICT(pack, collection, lang) DO UPDATE SET
                     hash = excluded.hash, release_number = excluded.release_number,
                     body = excluded.body, fetched_at = excluded.fetched_at",
                params![c.pack, c.collection, c.lang, c.hash, c.release, c.body],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A collection in one language from every pack that has it and is switched on, ordered by
    /// pack code: the unit the app reads (the core pack and the expansions add up).
    pub fn cached_for(&self, collection: &str, lang: &str) -> Result<Vec<CachedCollection>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.pack, c.collection, c.lang, c.hash, c.release_number, c.body
             FROM content_cache c LEFT JOIN content_packs p ON p.code = c.pack
             WHERE c.collection = ?1 AND c.lang = ?2 AND COALESCE(p.enabled, 1) = 1
             ORDER BY c.pack",
        )?;
        let rows = stmt.query_map(params![collection, lang], |r| {
            Ok(CachedCollection { pack: r.get(0)?, collection: r.get(1)?, lang: r.get(2)?, hash: r.get(3)?, release: r.get(4)?, body: r.get(5)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn cached(&self, pack: &str, collection: &str, lang: &str) -> Result<Option<CachedCollection>> {
        Ok(self
            .conn
            .query_row(
                "SELECT pack, collection, lang, hash, release_number, body FROM content_cache
                 WHERE pack = ?1 AND collection = ?2 AND lang = ?3",
                params![pack, collection, lang],
                |r| {
                    Ok(CachedCollection { pack: r.get(0)?, collection: r.get(1)?, lang: r.get(2)?, hash: r.get(3)?, release: r.get(4)?, body: r.get(5)? })
                },
            )
            .optional()?)
    }

    /// Every cached collection without its body, ordered by pack, collection and language.
    pub fn cache_index(&self) -> Result<Vec<CacheEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pack, collection, lang, hash, release_number FROM content_cache ORDER BY pack, collection, lang")?;
        let rows = stmt.query_map([], |r| {
            Ok(CacheEntry { pack: r.get(0)?, collection: r.get(1)?, lang: r.get(2)?, hash: r.get(3)?, release: r.get(4)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Deletes cached collections of a pack, narrowed to one collection and / or one language.
    /// Returns how many were deleted.
    pub fn delete_cached(&self, pack: &str, collection: Option<&str>, lang: Option<&str>) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM content_cache WHERE pack = ?1 AND (?2 IS NULL OR collection = ?2) AND (?3 IS NULL OR lang = ?3)",
            params![pack, collection, lang],
        )?)
    }

    pub fn set_pack_enabled(&self, code: &str, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO content_packs (code, enabled) VALUES (?1, ?2) ON CONFLICT(code) DO UPDATE SET enabled = excluded.enabled",
            params![code, enabled],
        )?;
        Ok(())
    }

    /// A pack with no stored switch is on.
    pub fn pack_enabled(&self, code: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT enabled FROM content_packs WHERE code = ?1", [code], |r| r.get::<_, bool>(0))
            .optional()?
            .unwrap_or(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection(pack: &str, name: &str, lang: &str, hash: &str, release: i64) -> CachedCollection {
        CachedCollection {
            pack: pack.into(),
            collection: name.into(),
            lang: lang.into(),
            hash: hash.into(),
            release,
            body: format!(r#"{{"entries":["{name}-{lang}-{hash}"]}}"#),
        }
    }

    #[test]
    fn settings_are_stored_replaced_and_deleted() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(s.setting("installation.token").unwrap(), None);
        s.set_setting("installation.token", "rcs_one").unwrap();
        s.set_setting("installation.token", "rcs_two").unwrap();
        assert_eq!(s.setting("installation.token").unwrap().as_deref(), Some("rcs_two"));
        assert!(s.delete_setting("installation.token").unwrap());
        assert!(!s.delete_setting("installation.token").unwrap());
        assert_eq!(s.setting("installation.token").unwrap(), None);
    }

    #[test]
    fn a_collection_is_cached_replaced_and_read_back_untouched() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(s.cached("core", "weapons", "ru").unwrap(), None);
        let first = collection("core", "weapons", "ru", "aa", 3);
        s.put_cached(std::slice::from_ref(&first)).unwrap();
        assert_eq!(s.cached("core", "weapons", "ru").unwrap(), Some(first));
        let newer = collection("core", "weapons", "ru", "bb", 4);
        s.put_cached(std::slice::from_ref(&newer)).unwrap();
        assert_eq!(s.cached("core", "weapons", "ru").unwrap(), Some(newer));
        // The same collection in another language or pack is a different row.
        assert_eq!(s.cached("core", "weapons", "en").unwrap(), None);
        assert_eq!(s.cached("dlc1", "weapons", "ru").unwrap(), None);
    }

    #[test]
    fn the_index_lists_what_is_cached_without_the_bodies() {
        let s = Storage::open_in_memory().unwrap();
        s.put_cached(&[collection("core", "weapons", "en", "h2", 5), collection("core", "armor", "ru", "h1", 5), collection("core", "weapons", "ru", "h3", 5)])
            .unwrap();
        let keys: Vec<_> = s.cache_index().unwrap().into_iter().map(|e| (e.collection, e.lang, e.hash)).collect();
        assert_eq!(
            keys,
            vec![
                ("armor".to_string(), "ru".to_string(), "h1".to_string()),
                ("weapons".to_string(), "en".to_string(), "h2".to_string()),
                ("weapons".to_string(), "ru".to_string(), "h3".to_string()),
            ]
        );
    }

    #[test]
    fn deleting_can_be_narrowed_to_a_collection_or_a_language() {
        let s = Storage::open_in_memory().unwrap();
        let all = [
            collection("core", "weapons", "ru", "a", 1),
            collection("core", "weapons", "en", "b", 1),
            collection("core", "armor", "ru", "c", 1),
            collection("dlc1", "weapons", "ru", "d", 1),
        ];
        s.put_cached(&all).unwrap();
        assert_eq!(s.delete_cached("core", Some("weapons"), Some("en")).unwrap(), 1);
        assert_eq!(s.delete_cached("core", None, Some("ru")).unwrap(), 2);
        assert_eq!(s.cache_index().unwrap().len(), 1, "only the other pack is left");
        assert_eq!(s.delete_cached("dlc1", None, None).unwrap(), 1);
        assert!(s.cache_index().unwrap().is_empty());
    }

    #[test]
    fn a_collection_is_read_from_every_enabled_pack() {
        let s = Storage::open_in_memory().unwrap();
        s.put_cached(&[
            collection("core", "weapons", "ru", "a", 1),
            collection("dlc1", "weapons", "ru", "b", 1),
            collection("dlc1", "weapons", "en", "c", 1),
            collection("core", "armor", "ru", "d", 1),
        ])
        .unwrap();
        let packs = |s: &Storage| s.cached_for("weapons", "ru").unwrap().into_iter().map(|c| c.pack).collect::<Vec<_>>();
        assert_eq!(packs(&s), vec!["core", "dlc1"]);
        s.set_pack_enabled("dlc1", false).unwrap();
        assert_eq!(packs(&s), vec!["core"]);
        assert!(s.cached_for("weapons", "en").unwrap().is_empty(), "dlc1 is off in every language");
    }

    #[test]
    fn a_pack_is_on_until_switched_off_and_its_data_stays() {
        let s = Storage::open_in_memory().unwrap();
        assert!(s.pack_enabled("core").unwrap());
        s.put_cached(&[collection("core", "weapons", "ru", "a", 1)]).unwrap();
        s.set_pack_enabled("core", false).unwrap();
        assert!(!s.pack_enabled("core").unwrap());
        assert!(s.cached("core", "weapons", "ru").unwrap().is_some(), "switching off keeps the download");
        s.set_pack_enabled("core", true).unwrap();
        assert!(s.pack_enabled("core").unwrap());
    }

    #[test]
    fn an_older_database_gets_the_sync_tables_and_keeps_its_characters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("red-companion.db");
        crate::tests::old_database(&path);
        let s = Storage::open(&path).unwrap();
        assert_eq!(s.list_characters(crate::CharacterKind::Pc).unwrap().len(), 1);
        s.set_setting("content.mode", "online").unwrap();
        s.put_cached(&[collection("core", "weapons", "ru", "a", 1)]).unwrap();
        assert_eq!(s.setting("content.mode").unwrap().as_deref(), Some("online"));
    }
}
