//! Single access layer for reference data. The UI and the rules never know
//! whether entries come from the built-in snapshot, a local cache or the API.

use thiserror::Error;

use crate::catalog::{
    Armor, Collection, CollectionVersion, CoverMaterial, CriticalInjury, Shield,
};

#[derive(Debug, Error)]
pub enum DataError {
    #[error("collection {name:?} is malformed: {source}")]
    Malformed { name: &'static str, source: serde_json::Error },
}

pub type DataResult<T> = Result<T, DataError>;

pub trait DataSource {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>>;
    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>>;
    fn armor(&self) -> DataResult<Collection<Armor>>;
    fn shields(&self) -> DataResult<Collection<Shield>>;

    /// Versions of every collection, the "manifest" compared during sync.
    fn versions(&self) -> DataResult<Vec<CollectionVersion>> {
        Ok(vec![
            self.critical_injuries()?.version_info(),
            self.cover_materials()?.version_info(),
            self.armor()?.version_info(),
            self.shields()?.version_info(),
        ])
    }
}

/// Snapshot compiled into the binary: works offline, no server needed.
#[derive(Debug, Default, Clone, Copy)]
pub struct SeedDataSource;

fn parse<T: serde::de::DeserializeOwned>(
    name: &'static str,
    json: &str,
) -> DataResult<Collection<T>> {
    serde_json::from_str(json).map_err(|source| DataError::Malformed { name, source })
}

impl DataSource for SeedDataSource {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>> {
        parse("critical_injuries", include_str!("../data/seed/critical_injuries.json"))
    }
    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>> {
        parse("cover_materials", include_str!("../data/seed/cover_materials.json"))
    }
    fn armor(&self) -> DataResult<Collection<Armor>> {
        parse("armor", include_str!("../data/seed/armor.json"))
    }
    fn shields(&self) -> DataResult<Collection<Shield>> {
        parse("shields", include_str!("../data/seed/shields.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_files_parse_and_names_match() {
        let ds = SeedDataSource;
        assert_eq!(ds.critical_injuries().unwrap().name, "critical_injuries");
        assert_eq!(ds.cover_materials().unwrap().name, "cover_materials");
        assert_eq!(ds.armor().unwrap().name, "armor");
        assert_eq!(ds.shields().unwrap().name, "shields");
    }

    #[test]
    fn manifest_lists_all_collections() {
        let names: Vec<_> = SeedDataSource.versions().unwrap().into_iter().map(|v| v.name).collect();
        assert_eq!(names, ["critical_injuries", "cover_materials", "armor", "shields"]);
    }

    #[test]
    fn works_through_trait_object() {
        let ds: Box<dyn DataSource> = Box::new(SeedDataSource);
        assert!(ds.versions().is_ok());
    }

    #[test]
    fn malformed_json_is_reported() {
        let err = parse::<Armor>("armor", "{").unwrap_err();
        assert!(err.to_string().contains("armor"));
    }

    /// Every i18n key referenced by seed data exists in both locale files.
    #[test]
    fn seed_keys_exist_in_both_locales() {
        let ds = SeedDataSource;
        let mut keys = vec![];
        for e in ds.critical_injuries().unwrap().entries {
            keys.push(e.name_key);
            keys.push(e.effect_key);
        }
        keys.extend(ds.cover_materials().unwrap().entries.into_iter().map(|e| e.name_key));
        keys.extend(ds.armor().unwrap().entries.into_iter().map(|e| e.name_key));
        keys.extend(ds.shields().unwrap().entries.into_iter().map(|e| e.name_key));

        for lang in ["ru", "en"] {
            let path = format!("{}/../../src/i18n/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let root: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            for key in &keys {
                let found = key.split('.').try_fold(&root, |v, part| v.get(part));
                assert!(found.is_some_and(|v| v.is_string()), "{lang}: missing {key}");
            }
        }
    }

    #[test]
    fn seed_data_passes_invariants() {
        use crate::catalog::*;
        let ds = SeedDataSource;
        assert_eq!(validate_critical_injuries(&ds.critical_injuries().unwrap()), Ok(()));
        assert_eq!(validate_cover_materials(&ds.cover_materials().unwrap()), Ok(()));
        assert_eq!(validate_armor(&ds.armor().unwrap()), Ok(()));
        assert_eq!(validate_shields(&ds.shields().unwrap()), Ok(()));
    }

    /// Spot checks against the book tables (p. 182, 184-185, 187-188).
    #[test]
    fn seed_values_match_book() {
        use crate::catalog::*;
        let ds = SeedDataSource;
        let cover = ds.cover_materials().unwrap();
        let hp = |id: &str| cover.entries.iter().find(|e| e.id == id).unwrap().hp;
        assert_eq!((hp("concrete_thick"), hp("wood_thin"), hp("metal_thick")), (25, 5, 50));
        assert!(cover.entries.iter().all(|e| e.id != "drywall_thin"));

        let armor = ds.armor().unwrap();
        let a = |id: &str| armor.entries.iter().find(|e| e.id == id).unwrap().clone();
        assert_eq!((a("kevlar").sp, a("kevlar").penalty), (7, 0));
        assert_eq!((a("metalgear").sp, a("metalgear").penalty), (18, 4));

        let shields = ds.shields().unwrap();
        assert_eq!(shields.entries[0].hp, Some(10));
        assert_eq!(shields.entries[1].hp, None);

        let inj = ds.critical_injuries().unwrap();
        let broken_leg = inj.entries.iter().find(|e| e.id == "broken_leg").unwrap();
        assert_eq!((broken_leg.area, broken_leg.roll), (BodyArea::Body, 8));
        assert_eq!(broken_leg.first_aid, [TreatmentOption { skill: TreatmentSkill::Paramedic, dv: 13 }]);
        assert_eq!(broken_leg.treatment.len(), 2);
        let dsp = inj.entries.iter().filter(|e| e.death_save_penalty).count();
        assert_eq!(dsp, 11); // 5 body + 6 head rows
    }
}
