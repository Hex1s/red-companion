//! The app's `DataSource` over the local cache: the mechanics (injuries, skills, rules, range tables,
//! roles, magazines, price categories, specialties) come from the snapshot compiled into the app, the
//! content (weapons, armor, NPCs, ...) from what was downloaded. Reading is synchronous and local;
//! getting the data into the cache is `sync::run`. A collection that was never downloaded is empty.

use rules_core::catalog::{
    Ammo, Armor, Article, ClothingStyle, Collection, CoverMaterial, CriticalInjury, GearItem, MagazineRow, MartialArt, NpcTemplate, PriceCategory,
    RangeTable, Role, Shield, Skill, Specialty, StreetDrug, Weapon, WeaponMod,
};
use rules_core::data_source::{DataError, DataResult, DataSource, Lang, SeedDataSource};
use serde::de::DeserializeOwned;
use std::sync::Mutex;

use storage::{CachedCollection, Storage};

/// Where downloaded collections are read from. Implemented for the database itself and for the
/// mutex the app keeps it in; the latter locks only for the read, so it must not be used while the
/// caller already holds that lock.
pub trait CacheReader {
    fn cached_for(&self, collection: &str, lang: &str) -> Result<Vec<CachedCollection>, String>;
}

impl CacheReader for Storage {
    fn cached_for(&self, collection: &str, lang: &str) -> Result<Vec<CachedCollection>, String> {
        Storage::cached_for(self, collection, lang).map_err(|e| e.to_string())
    }
}

impl CacheReader for Mutex<Storage> {
    fn cached_for(&self, collection: &str, lang: &str) -> Result<Vec<CachedCollection>, String> {
        let storage = self.lock().map_err(|_| "the database lock is poisoned".to_string())?;
        Storage::cached_for(&storage, collection, lang).map_err(|e| e.to_string())
    }
}

pub struct CachedSource<'a, R: CacheReader + ?Sized> {
    cache: &'a R,
    lang: Lang,
    seed: SeedDataSource,
}

impl<'a, R: CacheReader + ?Sized> CachedSource<'a, R> {
    pub fn new(cache: &'a R, lang: Lang) -> Self {
        Self { cache, lang, seed: SeedDataSource::new(lang) }
    }

    /// The downloaded collection: the copies from every enabled pack, one after another. Nothing
    /// downloaded: an empty collection.
    fn server<T: DeserializeOwned>(&self, name: &'static str) -> DataResult<Collection<T>> {
        let lang = match self.lang {
            Lang::Ru => "ru",
            Lang::En => "en",
        };
        let rows = self.cache.cached_for(name, lang).map_err(|reason| DataError::Cache { name, reason })?;
        let mut merged: Option<Collection<T>> = None;
        for row in rows {
            let part: Collection<T> = serde_json::from_str(&row.body).map_err(|source| DataError::Malformed { name, source })?;
            match &mut merged {
                None => merged = Some(part),
                Some(all) => {
                    all.version = all.version.max(part.version);
                    all.entries.extend(part.entries);
                }
            }
        }
        Ok(merged.unwrap_or_else(|| Collection { name: name.to_string(), version: 0, entries: Vec::new() }))
    }
}

impl<R: CacheReader + ?Sized> DataSource for CachedSource<'_, R> {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>> {
        self.seed.critical_injuries()
    }
    fn skills(&self) -> DataResult<Collection<Skill>> {
        self.seed.skills()
    }
    fn roles(&self) -> DataResult<Collection<Role>> {
        self.seed.roles()
    }
    fn articles(&self) -> DataResult<Collection<Article>> {
        self.seed.articles()
    }
    fn range_tables(&self) -> DataResult<Collection<RangeTable>> {
        self.seed.range_tables()
    }
    fn magazines(&self) -> DataResult<Collection<MagazineRow>> {
        self.seed.magazines()
    }
    fn price_categories(&self) -> DataResult<Collection<PriceCategory>> {
        self.seed.price_categories()
    }
    fn specialties(&self) -> DataResult<Collection<Specialty>> {
        self.seed.specialties()
    }

    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>> {
        self.server("cover_materials")
    }
    fn armor(&self) -> DataResult<Collection<Armor>> {
        self.server("armor")
    }
    fn shields(&self) -> DataResult<Collection<Shield>> {
        self.server("shields")
    }
    fn weapons(&self) -> DataResult<Collection<Weapon>> {
        self.server("weapons")
    }
    fn npc_templates(&self) -> DataResult<Collection<NpcTemplate>> {
        self.server("npc_templates")
    }
    fn martial_arts(&self) -> DataResult<Collection<MartialArt>> {
        self.server("martial_arts")
    }
    fn gear(&self) -> DataResult<Collection<GearItem>> {
        self.server("gear")
    }
    fn ammo(&self) -> DataResult<Collection<Ammo>> {
        self.server("ammo")
    }
    fn weapon_mods(&self) -> DataResult<Collection<WeaponMod>> {
        self.server("weapon_mods")
    }
    fn clothing(&self) -> DataResult<Collection<ClothingStyle>> {
        self.server("clothing")
    }
    fn street_drugs(&self) -> DataResult<Collection<StreetDrug>> {
        self.server("street_drugs")
    }
}

#[cfg(test)]
mod tests {
    use storage::CachedCollection;

    use super::*;

    /// A cached `armor` collection with the given entries, in the shape the server sends.
    fn cached_armor(pack: &str, lang: &str, entries: &[Armor]) -> CachedCollection {
        let body = serde_json::json!({ "name": "armor", "version": 7, "entries": entries });
        CachedCollection { pack: pack.into(), collection: "armor".into(), lang: lang.into(), hash: "h".into(), release: 1, body: body.to_string() }
    }

    fn seed_armor() -> Vec<Armor> {
        SeedDataSource::new(Lang::Ru).armor().unwrap().entries
    }

    #[test]
    fn a_collection_never_downloaded_is_empty() {
        let storage = Storage::open_in_memory().unwrap();
        let armor = CachedSource::new(&storage, Lang::Ru).armor().unwrap();
        assert_eq!((armor.entries.len(), armor.version, armor.name.as_str()), (0, 0, "armor"));
    }

    #[test]
    fn downloaded_data_is_what_the_app_reads() {
        let storage = Storage::open_in_memory().unwrap();
        let seed = seed_armor();
        assert!(seed.len() >= 2, "the test needs at least two embedded entries");
        storage.put_cached(&[cached_armor("core", "ru", &seed[..1])]).unwrap();
        let armor = CachedSource::new(&storage, Lang::Ru).armor().unwrap();
        assert_eq!((armor.entries.len(), armor.version), (1, 7));
        // The other language was not downloaded.
        assert!(CachedSource::new(&storage, Lang::En).armor().unwrap().entries.is_empty());
    }

    #[test]
    fn packs_add_up_and_a_switched_off_pack_is_left_out() {
        let storage = Storage::open_in_memory().unwrap();
        let seed = seed_armor();
        storage.put_cached(&[cached_armor("core", "ru", &seed[..1]), cached_armor("dlc1", "ru", &seed[1..2])]).unwrap();
        let count = |s: &Storage| CachedSource::new(s, Lang::Ru).armor().unwrap().entries.len();
        assert_eq!(count(&storage), 2);
        storage.set_pack_enabled("dlc1", false).unwrap();
        assert_eq!(count(&storage), 1);
    }

    #[test]
    fn a_broken_copy_is_an_error_not_an_empty_list() {
        let storage = Storage::open_in_memory().unwrap();
        let mut broken = cached_armor("core", "ru", &[]);
        broken.body = r#"{"name":"armor","version":1,"entries":[{"nonsense":true}]}"#.into();
        storage.put_cached(&[broken]).unwrap();
        assert!(matches!(CachedSource::new(&storage, Lang::Ru).armor(), Err(DataError::Malformed { name: "armor", .. })));
    }

    #[test]
    fn the_app_reads_through_its_mutex() {
        let storage = Mutex::new(Storage::open_in_memory().unwrap());
        let seed = seed_armor();
        storage.lock().unwrap().put_cached(&[cached_armor("core", "ru", &seed[..1])]).unwrap();
        assert_eq!(CachedSource::new(&storage, Lang::Ru).armor().unwrap().entries.len(), 1);
    }

    #[test]
    fn the_mechanics_always_come_from_the_embedded_snapshot() {
        let storage = Storage::open_in_memory().unwrap();
        let source = CachedSource::new(&storage, Lang::Ru);
        assert_eq!(source.skills().unwrap().entries.len(), SeedDataSource::new(Lang::Ru).skills().unwrap().entries.len());
        assert!(!source.critical_injuries().unwrap().entries.is_empty());
    }
}
