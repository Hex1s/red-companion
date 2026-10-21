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
}
